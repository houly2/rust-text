use gpui::*;

use crate::theme_manager::ActiveTheme;

use super::icons::Icons;

#[derive(IntoElement)]
pub struct IconButton {
    icon: Icons,
    selected: bool,
}

impl IconButton {
    pub fn new(icon: Icons) -> Self {
        Self {
            icon,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl RenderOnce for IconButton {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .w_6()
            .h_6()
            .p_1()
            .text_sm()
            .cursor(CursorStyle::PointingHand)
            .hover(|el| {
                el.rounded_md()
                    .bg(cx.theme().background.blend(rgba(0x00000088)))
            })
            .child(
                svg()
                    .size_4()
                    .flex_none()
                    .path(self.icon.path())
                    .text_color(if self.selected {
                        cx.theme().scroll_bar_cursor_highlight
                    } else {
                        cx.theme().editor_text
                    }),
            )
    }
}
