use std::path::PathBuf;

use gpui::*;

use crate::{
    settings_manager::CurrentSettings, theme_manager::ActiveTheme,
    views::text_input::text_input::TextInput,
};

pub struct TitleBar {
    text_input: WeakView<TextInput>,
}

impl TitleBar {
    pub fn new(text_input: WeakView<TextInput>) -> Self {
        Self { text_input }
    }

    fn file_name<'a>(&self, path: &'a Option<PathBuf>) -> &'a str {
        if let Some(path) = path {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name) = file_name.to_str() {
                    return file_name;
                }
            }
        }

        ""
    }

    fn title(&self, cx: &ViewContext<Self>) -> String {
        let Some(text_input) = self.text_input.upgrade() else {
            return "".into();
        };

        let text_input = text_input.read(cx);

        format!(
            "{}{}",
            if text_input.is_dirty() { "🞄" } else { "" },
            self.file_name(text_input.file_path())
        )
    }
}

impl Render for TitleBar {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .h(px(32.))
            .flex_none()
            .font_family(cx.settings().font_family)
            .text_sm()
            .text_color(cx.theme().editor_text)
            .child(
                div()
                    .flex()
                    .h_full()
                    .w_full()
                    .justify_center()
                    .items_center()
                    .child(self.title(cx)),
            )
    }
}
