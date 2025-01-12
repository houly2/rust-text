use assets::Assets;
use db::{DbConnection, DB};
use editor::editor::*;
use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use gpui::*;
use settings_manager::SettingsManager;
use std::path::PathBuf;
use views::text_input::text_input::*;

mod assets;
mod db;
mod editor;
mod settings_manager;
mod theme_manager;
mod views;

use crate::theme_manager::ActiveTheme;

actions!(set_menus, [Quit, Hide, HideOthers, ShowAll, FileNew, Open]);

fn bounds_for_path(path: &Option<PathBuf>, cx: &AppContext) -> WindowBounds {
    if let Some(path) = path {
        if let Some(positions) = cx.db_connection().window_position(path.clone()) {
            for display in cx.displays() {
                let Ok(display_uuid) = display.uuid() else {
                    continue;
                };
                for pos in &positions {
                    if pos.display_id == display_uuid {
                        return WindowBounds::Windowed(bounds(
                            point(px(pos.bounds.origin.x), px(pos.bounds.origin.y)),
                            size(px(pos.bounds.size.width), px(pos.bounds.size.height)),
                        ));
                    }
                }
            }
        }
    }

    WindowBounds::Windowed(Bounds::centered(None, size(px(400.), px(320.)), cx))
}

fn open_window(path: Option<PathBuf>, cx: &mut AppContext) {
    let window = cx
        .open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(9.0), px(9.0))),
                }),
                window_bounds: Some(bounds_for_path(&path, cx)),
                window_min_size: Some(size(px(200.), px(160.))),
                ..Default::default()
            },
            |cx| {
                let editor = cx.new_view(|cx| {
                    let mut element = Editor::new(cx);

                    if let Some(path) = path {
                        element.read_file(&path, cx);
                    }

                    element
                });

                cx.new_view(|cx| TextEditor::new(editor, cx.focus_handle(), cx))
            },
        )
        .unwrap();

    window
        .update(cx, |view, cx| {
            cx.focus_view(&view.editor);
            cx.activate(true);
        })
        .unwrap();
}

fn open_file(cx: &mut AppContext) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
    });

    cx.spawn(|cx| async move {
        match Flatten::flatten(paths.await.map_err(|e| e.into())) {
            Ok(Some(paths)) => {
                if let Some(path) = paths.first() {
                    cx.update(|cx| {
                        cx.add_recent_document(path);
                        open_window(Some(path.to_path_buf()), cx);
                    })
                    .ok();
                }
            }
            Ok(None) => {}
            Err(_) => {}
        }
    })
    .detach();
}

struct TextEditor {
    editor: View<Editor>,
    focus_handle: FocusHandle,
    _settings_manager: SettingsManager,
}

impl TextEditor {
    fn new(editor: View<Editor>, focus_handle: FocusHandle, cx: &mut AppContext) -> Self {
        Self {
            editor,
            focus_handle,
            _settings_manager: SettingsManager::new(cx),
        }
    }
}

impl FocusableView for TextEditor {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextEditor {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .size_full()
            .child(self.editor.clone())
    }
}

#[derive(Clone)]
struct OpenListener(UnboundedSender<Vec<String>>);

impl OpenListener {
    pub fn new() -> (Self, UnboundedReceiver<Vec<String>>) {
        let (tx, rx) = mpsc::unbounded();
        (OpenListener(tx), rx)
    }

    pub fn open_urls(&self, urls: Vec<String>) {
        let urls = urls
            .iter()
            .filter_map(|url| url.strip_prefix("file://"))
            .map(|str| str.to_string())
            .collect();
        self.0.unbounded_send(urls).ok();
    }
}

fn main() {
    let app = App::new().with_assets(Assets {});

    let (open_listener, mut open_rx) = OpenListener::new();

    app.on_open_urls({
        let open_listener = open_listener.clone();
        move |urls| open_listener.open_urls(urls)
    });

    app.run(move |cx: &mut AppContext| {
        _ = DB::register_global(cx);

        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", Open, None),
            KeyBinding::new("cmd-s", Save, None),
            KeyBinding::new("shift-cmd-s", SaveAs, None),
            KeyBinding::new("cmd-h", Hide, None),
            KeyBinding::new("alt-cmd-h", HideOthers, None),
            KeyBinding::new("cmd-n", FileNew, None),
            KeyBinding::new("cmd-w", WindowClose, None),
            KeyBinding::new("cmd-m", Minimize, None),
        ]);

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &FileNew, cx| open_window(None, cx));
        cx.on_action(|_: &Open, cx| open_file(cx));

        cx.set_menus(vec![
            Menu {
                name: "Text".into(),
                items: vec![
                    MenuItem::action("About text...", About),
                    MenuItem::separator(),
                    MenuItem::action("Hide", Hide),
                    MenuItem::action("Hide Others", HideOthers),
                    MenuItem::action("Show All", ShowAll),
                    MenuItem::action("Quit", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("New", FileNew),
                    MenuItem::separator(),
                    MenuItem::action("Open", Open),
                    MenuItem::separator(),
                    MenuItem::action("Save", Save),
                    MenuItem::action("Save As", SaveAs),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Undo", Undo, OsAction::Undo),
                    MenuItem::os_action("Redo", Redo, OsAction::Redo),
                    MenuItem::separator(),
                    MenuItem::os_action("Cut", Cut, OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, OsAction::Paste),
                ],
            },
            Menu {
                name: "Selection".into(),
                items: vec![MenuItem::os_action(
                    "Select All",
                    SelectAll,
                    OsAction::SelectAll,
                )],
            },
            Menu {
                name: "Window".into(),
                items: vec![MenuItem::action("Minimize", Minimize)],
            },
        ]);

        if let Some(urls) = open_rx.try_next().ok().flatten() {
            for url in urls {
                open_window(Some(PathBuf::from(url)), cx);
            }
        } else {
            #[cfg(debug_assertions)]
            {
                let path = PathBuf::from("/Users/philipwagner/Downloads/test.txt");
                open_window(Some(path), cx);
            }

            #[cfg(not(debug_assertions))]
            {
                open_window(None, cx);
            }
        }

        cx.spawn(move |cx| async move {
            while let Some(urls) = open_rx.next().await {
                cx.update(|cx| {
                    for url in urls {
                        open_window(Some(PathBuf::from(url)), cx);
                    }
                })
                .ok();
            }
        })
        .detach();
    });
}
