use gpui::*;

mod blink_manager;
mod text_element;
mod text_input;

use crate::text_input::*;

actions!(set_menus, [Quit]);

fn quit(_: &Quit, cx: &mut AppContext) {
    cx.quit();
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
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("alt-left", MoveToWordStart, None),
            KeyBinding::new("alt-right", MoveToWordEnd, None),
            KeyBinding::new("cmd-left", MoveToLineStart, None),
            KeyBinding::new("cmd-right", MoveToLineEnd, None),
            KeyBinding::new("shift-alt-left", SelectWordStart, None),
            KeyBinding::new("shift-alt-right", SelectWordEnd, None),
            KeyBinding::new("shift-cmd-left", SelectLineStart, None),
            KeyBinding::new("shift-cmd-right", SelectLineEnd, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(9.0), px(9.0))),
                    }),
                    ..Default::default()
                },
                |cx| {
                    let text_input = cx.new_view(|cx| TextInput::new(cx));
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
        cx.set_menus(vec![Menu {
            name: "set_menus".into(),
            items: vec![MenuItem::action("Quit", Quit)],
        }]);

        window
            .update(cx, |view, cx| {
                cx.focus_view(&view.text_input);
                cx.activate(true);
            })
            .unwrap();
    });
}
