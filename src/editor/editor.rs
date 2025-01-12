use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::*;

use crate::{
    db::DbConnection,
    settings_manager::CurrentSettings,
    theme_manager::ActiveTheme,
    views::text_input::text_input::{TextInput, TextInputMode},
};

use super::{
    modal_manager::ModalManager, search::SearchView, status_bar::StatusBar,
    theme_selector::ThemeSelector, title_bar::TitleBar,
};

actions!(
    set_menus,
    [
        Save,
        SaveAs,
        About,
        WindowClose,
        Minimize,
        ToggleTheme,
        Search
    ]
);

pub struct Editor {
    text_input: View<TextInput>,
    title_bar: View<TitleBar>,
    status_bar: View<StatusBar>,
    modal_manager: View<ModalManager>,
    search_view: View<SearchView>,
    bounds_save_task_queue: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let handle = cx.view().downgrade();
        cx.on_window_should_close(move |cx| {
            handle
                .update(cx, |this, cx| this.allowed_to_close_window(cx))
                .unwrap_or(true)
        });

        cx.bind_keys([KeyBinding::new("cmd-t", ToggleTheme, None)]);
        cx.bind_keys([KeyBinding::new("cmd-f", Search, None)]);

        let text_input = cx.new_view(|cx| TextInput::new(TextInputMode::Full, cx));

        let weak_handle = text_input.downgrade();
        let title_bar = cx.new_view(|_| TitleBar::new(weak_handle.clone()));
        let status_bar = cx.new_view(|_| StatusBar::new(weak_handle.clone()));
        let search_view = cx.new_view(|cx| SearchView::new(weak_handle.clone(), cx));

        let _subscriptions = vec![cx.observe_window_bounds(|this, cx| {
            if this.bounds_save_task_queue.is_some() {
                return;
            }

            this.bounds_save_task_queue = Some(cx.spawn(|this, mut cx| async move {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                this.update(&mut cx, |this, cx| {
                    if let Some(display) = cx.display() {
                        if let Ok(display_uuid) = display.uuid() {
                            let window_bounds = cx.window_bounds();
                            let bounds = match window_bounds {
                                WindowBounds::Fullscreen(bounds) => bounds,
                                WindowBounds::Windowed(bounds) => bounds,
                                WindowBounds::Maximized(bounds) => bounds,
                            };
                            let text_input = this.text_input.read(cx);
                            let path = text_input
                                .file_path()
                                .as_ref()
                                .map(|p| p.clone())
                                .unwrap_or(Path::new("").to_path_buf());

                            cx.db_connection()
                                .update_window_position(&path, display_uuid, bounds)
                        }
                    }
                    this.bounds_save_task_queue.take();
                })
                .ok();
            }));
        })];

        Self {
            text_input,
            title_bar,
            status_bar,
            modal_manager: cx.new_view(ModalManager::new),
            search_view,
            bounds_save_task_queue: None,
            _subscriptions,
        }
    }

    fn save_handler(&mut self, _: &Save, cx: &mut ViewContext<Self>) {
        self.save(cx);
    }

    fn save(&mut self, cx: &mut ViewContext<Self>) {
        let text_input = self.text_input.read(cx);

        if let Some(path) = text_input.file_path() {
            self.save_file(path.into(), cx);
        } else {
            self.save_as(cx);
        }
    }

    fn save_as_handler(&mut self, _: &SaveAs, cx: &mut ViewContext<Self>) {
        self.save_as(cx);
    }

    fn save_as(&mut self, cx: &mut ViewContext<Self>) {
        self.prompt_for_new_path(
            |this, path, mut cx| {
                if let Some(path) = path {
                    cx.update(|cx| {
                        if let Some(this) = this.upgrade() {
                            this.update(cx, |this, cx| {
                                this.text_input
                                    .update(cx, |this, cx| this.set_file_path(path.into(), cx));
                                this.save_file(path.into(), cx);
                            });
                        }
                    })
                    .ok();
                } else {
                    // todo: handle
                }
            },
            cx,
        );
    }

    pub fn read_file(&mut self, path: &PathBuf, cx: &mut ViewContext<Self>) {
        if let Ok(new_content) = fs::read_to_string(path) {
            self.text_input.update(cx, |this, cx| {
                this.set_file_path(path.into(), cx);
                this.insert(new_content, cx);
                this.mark_dirty(false, cx);
                this.move_to(0, cx);
            });
        }
    }

    fn save_file(&mut self, path: PathBuf, cx: &mut ViewContext<Self>) {
        let text_input = self.text_input.read(cx);
        match fs::write(path, text_input.content.to_string()) {
            Ok(_) => self
                .text_input
                .update(cx, |this, cx| this.mark_dirty(false, cx)),
            Err(error) => println!("{:?}", error),
        }

        cx.notify();
    }

    fn prompt_for_new_path(
        &self,
        callback: impl FnOnce(WeakView<Self>, Option<&PathBuf>, AsyncWindowContext) + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        let path = cx.prompt_for_new_path(Path::new(""));

        cx.spawn(|weak_view, cx| async move {
            match Flatten::flatten(path.await.map_err(|e| e.into())) {
                Ok(Some(path)) => callback(weak_view, Some(&path), cx),
                Ok(None) => callback(weak_view, None, cx),
                Err(_) => callback(weak_view, None, cx),
            }
        })
        .detach();
    }

    fn about(&mut self, _: &About, cx: &mut ViewContext<Self>) {
        let message = "text";
        let detail = "a little #DecemberAdventure text editor";
        let prompt = cx.prompt(PromptLevel::Info, message, Some(detail), &["Ok"]);
        cx.foreground_executor()
            .spawn(async { prompt.await.ok() })
            .detach();
    }

    fn minimize(&mut self, _: &Minimize, cx: &mut ViewContext<Self>) {
        cx.minimize_window();
    }

    fn close_window(&mut self, _: &WindowClose, cx: &mut ViewContext<Self>) {
        if self.allowed_to_close_window(cx) {
            cx.remove_window();
        }
    }

    fn allowed_to_close_window(&self, cx: &mut ViewContext<Self>) -> bool {
        if !self.text_input.read(cx).is_dirty() {
            true
        } else {
            let message = "Close without saving?";
            let detail = "Data will be lost";
            let prompt = cx.prompt(
                PromptLevel::Info,
                message,
                Some(detail),
                &["Save", "Don't Save", "Abort"],
            );
            cx.spawn(|this, mut cx| async move {
                match prompt.await.ok() {
                    Some(0) => this.update(&mut cx, |this, cx| {
                        this.save(cx);
                        cx.remove_window();
                    }),
                    Some(1) => this.update(&mut cx, |_, cx| cx.remove_window()),
                    Some(2) | Some(3_usize..) | None => Ok(()),
                }
            })
            .detach();
            false
        }
    }

    fn toggle_modal(&mut self, _: &ToggleTheme, cx: &mut ViewContext<Self>) {
        self.modal_manager.update(cx, |modal_layer, cx| {
            modal_layer.toggle_modal(cx, ThemeSelector::new)
        });
    }

    fn open_search(&mut self, _: &Search, cx: &mut ViewContext<Self>) {
        self.search_view
            .update(cx, |search_view, cx| search_view.show(cx))
    }
}

impl Render for Editor {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .key_context("Editor")
            .on_action(cx.listener(Self::save_handler))
            .on_action(cx.listener(Self::save_as_handler))
            .on_action(cx.listener(Self::minimize))
            .on_action(cx.listener(Self::close_window))
            .on_action(cx.listener(Self::about))
            .on_action(cx.listener(Self::toggle_modal))
            .on_action(cx.listener(Self::open_search))
            .child(self.title_bar.clone())
            .child(self.search_view.clone())
            .child(
                div()
                    .h_full()
                    .w_full()
                    .bg(cx.theme().editor_background)
                    .line_height(px(28.))
                    .text_size(px(18.))
                    .text_color(cx.theme().editor_text)
                    .font_family(cx.settings().font_family)
                    .child(self.text_input.clone()),
            )
            .child(self.status_bar.clone())
            .child(self.modal_manager.clone())
    }
}

impl FocusableView for Editor {
    fn focus_handle(&self, cx: &AppContext) -> FocusHandle {
        self.text_input.read(cx).focus_handle.clone()
    }
}
