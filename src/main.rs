use gpui::*;

mod blink_manager;
mod command;
mod lines;
mod scroll_manager;
mod text_element;
mod text_input;

use crate::text_input::*;

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
                cx.new_view(|cx| InputExample {
                    text_input: cx.new_view(|cx| TextInput::new(cx)),
                    focus_handle: cx.focus_handle(),
                })
            },
        )
        .unwrap();

    window
        .update(cx, |view, cx| {
            cx.focus_view(&view.text_input);
            cx.activate(true);
        })
        .unwrap();
}

struct InputExample {
    text_input: View<TextInput>,
    focus_handle: FocusHandle,
}

impl FocusableView for InputExample {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputExample {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .bg(rgb(0x1a1a29))
            .flex()
            .flex_col()
            .size_full()
            .child(div().h(px(32.)).flex_none())
            .child(self.text_input.clone())
    }
}

fn main() {
    App::new().run(|cx: &mut AppContext| {
        cx.bind_keys([
            KeyBinding::new("enter", NewLine, None),
            KeyBinding::new("cmd-enter", NewLineWithoutSplit, None),
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("shift-up", SelectUp, None),
            KeyBinding::new("shift-down", SelectDown, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("alt-left", MoveToWordStart, None),
            KeyBinding::new("alt-right", MoveToWordEnd, None),
            KeyBinding::new("cmd-left", MoveToLineStart, None),
            KeyBinding::new("cmd-right", MoveToLineEnd, None),
            KeyBinding::new("cmd-up", MoveToDocStart, None),
            KeyBinding::new("cmd-down", MoveToDocEnd, None),
            KeyBinding::new("shift-alt-left", SelectWordStart, None),
            KeyBinding::new("shift-alt-right", SelectWordEnd, None),
            KeyBinding::new("shift-cmd-left", SelectLineStart, None),
            KeyBinding::new("shift-cmd-right", SelectLineEnd, None),
            KeyBinding::new("shift-cmd-up", SelectDocStart, None),
            KeyBinding::new("shift-cmd-down", SelectDocEnd, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", Open, None),
            KeyBinding::new("cmd-h", Hide, None),
            KeyBinding::new("cmd-z", Undo, None),
            KeyBinding::new("cmd-n", FileNew, None),
            KeyBinding::new("cmd-shift-z", Redo, None),
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
                    let text_input = cx.new_view(|cx| {
                        // TextInput::new(cx)
                        let mut element = TextInput::new(cx);
                        // element.insert("Ä\nBBB BB BBBBB\nCCC".into(), cx);
                        // element.insert("Ä\nBΩB BB BBBBB\nCCC".into(), cx);
                        // element.insert("This is just ä Test! Ω≈ Haha.\nOtherwise i need to input this text all the time myself.\nAnd some more.".into(), cx);

                        element
                            .read_file(PathBuf::from("/Users/philipwagner/Downloads/test.txt"), cx);

                        return element;
                    });
                    cx.new_view(|cx| InputExample {
                        text_input,
                        focus_handle: cx.focus_handle(),
                    })
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
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::action("Undo", Undo),
                    MenuItem::action("Redo", Redo),
                ],
            },
            Menu {
                name: "Window".into(),
                items: vec![MenuItem::action("Minimize", Minimize)],
            },
        ]);

        window
            .update(cx, |view, cx| {
                cx.focus_view(&view.text_input);
                cx.activate(true);
            })
            .unwrap();
    });
}
