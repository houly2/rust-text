use gpui::*;

use crate::{
    settings_manager::CurrentSettings,
    theme_manager::ActiveTheme,
    views::{icons::Icons, text_input::text_input::TextInput, tooltip::Tooltip},
};

pub struct StatusBar {
    text_input: WeakView<TextInput>,
}

impl StatusBar {
    pub fn new(text_input: WeakView<TextInput>) -> Self {
        Self { text_input }
    }

    fn toggle_soft_wrap(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        let Some(text_input) = self.text_input.upgrade() else {
            return;
        };

        text_input.update(cx, |this, cx| {
            this.set_soft_wrap(!this.soft_wrap_enabled(), cx);
        });
    }

    fn soft_wrap_status(&self, cx: &mut ViewContext<Self>) -> bool {
        let Some(text_input) = self.text_input.upgrade() else {
            return false;
        };
        text_input.read(cx).soft_wrap_enabled()
    }

    fn selection_format(&self, cx: &mut ViewContext<Self>) -> String {
        let Some(text_input) = self.text_input.upgrade() else {
            return String::new();
        };

        let text_input = text_input.read(cx);

        let line_idx = text_input.content.char_to_line(text_input.cursor_offset());
        let line_char_idx = text_input.content.line_to_char(line_idx);
        let char_idx_in_line = text_input.cursor_offset() - line_char_idx;

        let selection = if text_input.selected_range.is_empty() {
            String::new()
        } else {
            let start_line_idx = text_input
                .content
                .char_to_line(text_input.selected_range.start);
            let end_line_idx = text_input
                .content
                .char_to_line(text_input.selected_range.end);
            let line_count = (start_line_idx as isize - end_line_idx as isize).abs();
            if line_count > 0 {
                format!(
                    " ({} lines, {} chars)",
                    line_count + 1,
                    (text_input.selected_range.start as isize
                        - text_input.selected_range.end as isize)
                        .abs()
                )
            } else {
                format!(
                    " ({} chars)",
                    (text_input.selected_range.start as isize
                        - text_input.selected_range.end as isize)
                        .abs()
                )
            }
        };

        format!("{}:{}{}", line_idx + 1, char_idx_in_line + 1, selection)
    }
}

impl Render for StatusBar {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .px(px(8.))
            .pt(px(2.))
            .pb(px(4.))
            .font_family(cx.settings().font_family)
            .text_sm()
            .text_color(cx.theme().editor_text)
            .child(
                div()
                    .px(px(4.))
                    .id("soft_wrap")
                    .on_click(cx.listener(Self::toggle_soft_wrap))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                svg()
                                    .size(px(10.))
                                    .flex_none()
                                    .path(if self.soft_wrap_status(cx) {
                                        Icons::RadioButtonChecked.path()
                                    } else {
                                        Icons::RadioButton.path()
                                    })
                                    .text_color(cx.theme().editor_text),
                            )
                            .child("Wrap"),
                    )
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.rounded(px(6.)).bg(cx.theme().hover_bg))
                    .tooltip(|cx| Tooltip::text("Toggle Soft Wrap", cx)),
            )
            .child(div().flex_grow())
            .child(self.selection_format(cx))
    }
}
