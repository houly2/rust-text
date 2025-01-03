use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::*;

use crate::{
    modal_manager::ModalManager, status_bar::StatusBar, text_input::TextInput,
    theme_selector::ThemeSelector, title_bar::TitleBar,
};

actions!(
    set_menus,
    [
        Open,
        Save,
        SaveAs,
        About,
        WindowClose,
        Minimize,
        ToggleTheme
    ]
);

pub struct Editor {
    text_input: View<TextInput>,
    title_bar: View<TitleBar>,
    status_bar: View<StatusBar>,
    modal_manager: View<ModalManager>,
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

        let text_input = cx.new_view(|cx| TextInput::new(TextInputMode::Full, cx));

        let weak_handle = text_input.downgrade();
        let title_bar = cx.new_view(|_| TitleBar::new(weak_handle.clone()));
        let status_bar = cx.new_view(|_| StatusBar::new(weak_handle.clone()));

        Self {
            text_input,
            title_bar,
            status_bar,
            modal_manager: cx.new_view(|cx| ModalManager::new(cx)),
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

    fn open(&mut self, _: &Open, cx: &mut ViewContext<Self>) {
        self.prompt_for_path(
            |this, path, mut cx| {
                if let Some(path) = path {
                    cx.update(|cx| {
                        cx.add_recent_document(path);
                        if let Some(this) = this.upgrade() {
                            this.update(cx, |this, cx| {
                                this.read_file(path, cx);
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
                this.insert(new_content.into(), cx);
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

    fn prompt_for_path(
        &self,
        callback: impl FnOnce(WeakView<Self>, Option<&PathBuf>, AsyncWindowContext) + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
        });

        cx.spawn(|weak_view, cx| async move {
            match Flatten::flatten(paths.await.map_err(|e| e.into())) {
                Ok(Some(paths)) => {
                    if let Some(path) = paths.first() {
                        callback(weak_view, Some(path), cx)
                    } else {
                        callback(weak_view, None, cx)
                    }
                }
                Ok(None) => callback(weak_view, None, cx),
                Err(_) => callback(weak_view, None, cx),
            }
        })
        .detach();
    }

    fn prompt_for_new_path(
        &self,
        callback: impl FnOnce(WeakView<Self>, Option<&PathBuf>, AsyncWindowContext) + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        let path = cx.prompt_for_new_path(Path::new("").into());

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
        let message = format!("text");
        let detail = format!("a little #DecemberAdventure text editor");
        let prompt = cx.prompt(PromptLevel::Info, &message, Some(&detail), &["Ok"]);
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
                &message,
                Some(&detail),
                &["Save", "Don't Save", "Abort"],
            );
            cx.spawn(|this, mut cx| async move {
                match prompt.await.ok() {
                    Some(0) => this.update(&mut cx, |this, cx| {
                        this.save(cx);
                        cx.remove_window();
                    }),
                    Some(1) => this.update(&mut cx, |_, cx| cx.remove_window()),
                    Some(2) | Some(3_usize..) | None => Ok({}),
                }
            })
            .detach();
            false
        }
    }

    fn toggle_modal(&mut self, _: &ToggleTheme, cx: &mut ViewContext<Self>) {
        self.modal_manager.update(cx, |modal_layer, cx| {
            modal_layer.toggle_modal(cx, move |cx| ThemeSelector::new(cx))
        });
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
            .on_action(cx.listener(Self::open))
            .on_action(cx.listener(Self::save_handler))
            .on_action(cx.listener(Self::save_as_handler))
            .on_action(cx.listener(Self::minimize))
            .on_action(cx.listener(Self::close_window))
            .on_action(cx.listener(Self::about))
            .on_action(cx.listener(Self::toggle_modal))
            .child(self.title_bar.clone())
            .child(self.text_input.clone())
            .child(self.status_bar.clone())
            .child(self.modal_manager.clone())
    }
}

impl FocusableView for Editor {
    fn focus_handle(&self, cx: &AppContext) -> FocusHandle {
        self.text_input.read(cx).focus_handle.clone()
    }
}
