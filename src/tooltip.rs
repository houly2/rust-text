use gpui::*;

use crate::{settings_manager::CurrentSettings, theme_manager::ActiveTheme};

pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    pub fn text(text: impl Into<SharedString>, cx: &mut WindowContext) -> AnyView {
        cx.new_view(|_| Self { text: text.into() }).into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .p_1()
            .bg(cx.theme().background)
            .rounded_md()
            .border_1()
            .border_color(cx.theme().scroll_bar_border)
            .font_family(cx.settings().font_family)
            .text_xs()
            .text_color(cx.theme().editor_text)
            .child(self.text.clone())
    }
}
