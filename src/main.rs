use gpui::*;
use settings_manager::SettingsManager;
use theme_manager::ThemeManager;

mod blink_manager;
mod command;
mod editor;
mod lines;
mod scroll_manager;
mod settings_manager;
mod status_bar;
mod text_element;
mod text_input;
mod theme_manager;
mod title_bar;

use crate::editor::*;
use crate::text_input::*;
use crate::theme_manager::ActiveTheme;

actions!(set_menus, [Quit, Hide, FileNew]);

fn quit(_: &Quit, cx: &mut AppContext) {
    cx.quit();
}

fn hide(_: &Hide, cx: &mut AppContext) {
    cx.hide();
}

fn file_new(_: &FileNew, cx: &mut AppContext) {
    let window = cx
        .open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(9.0), px(9.0))),
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(400.), px(320.)),
                    cx,
                ))),
                window_min_size: Some(size(px(200.), px(160.))),
                ..Default::default()
            },
            |cx| {
                cx.new_view(|cx| {
                    TextEditor::new(cx.new_view(|cx| Editor::new(cx)), cx.focus_handle(), cx)
                })
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

fn main() {
    App::new().run(|cx: &mut AppContext| {
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", Open, None),
            KeyBinding::new("cmd-s", Save, None),
            KeyBinding::new("shift-cmd-s", SaveAs, None),
            KeyBinding::new("cmd-h", Hide, None),
            KeyBinding::new("cmd-n", FileNew, None),
            KeyBinding::new("cmd-w", WindowClose, None),
            KeyBinding::new("cmd-m", Minimize, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(9.0), px(9.0))),
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(400.), px(320.)),
                        cx,
                    ))),
                    window_min_size: Some(size(px(200.), px(160.))),
                    ..Default::default()
                },
                |cx| {
                    let editor = cx.new_view(|cx| {
                        let mut element = Editor::new(cx);

                        #[cfg(debug_assertions)]
                        {
                            // element.insert("Ä\nBΩB BB BBBBB\nCCC".into(), cx);
                            // element.insert("This is just ä Test! Ω≈ Haha.\nOtherwise i need to input this text all the time myself.\nAnd some more.".into(), cx);
                            element.read_file(
                                &std::path::PathBuf::from("/Users/philipwagner/Downloads/test.txt"),
                                cx,
                            );
                        }

                        return element;
                    });
                    cx.new_view(|cx| TextEditor::new(editor, cx.focus_handle(), cx))
                },
            )
            .unwrap();

        cx.on_keyboard_layout_change({
            move |cx| {
                window.update(cx, |_, cx| cx.notify()).ok();
            }
        })
        .detach();

        cx.on_action(quit);
        cx.on_action(hide);
        cx.on_action(file_new);

        cx.set_menus(vec![
            Menu {
                name: "Text".into(),
                items: vec![
                    MenuItem::action("About text...", About),
                    MenuItem::separator(),
                    MenuItem::action("Hide", Hide),
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

        window
            .update(cx, |view, cx| {
                cx.focus_view(&view.editor);
                cx.activate(true);
            })
            .unwrap();
    });
}
