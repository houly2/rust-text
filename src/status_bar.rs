use gpui::*;

use crate::TextInput;

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
            this.set_soft_wrap(!this.soft_wrap_enabled(), cx)
        });
    }

    fn soft_wrap_status(&self, cx: &mut ViewContext<Self>) -> String {
        let Some(text_input) = self.text_input.upgrade() else {
            return "".to_string();
        };

        let text_input = text_input.read(cx);

        format!(
            "{} Wrap",
            if text_input.soft_wrap_enabled() {
                "◉"
            } else {
                "○"
            }
        )
    }

    fn selection_format(&self, cx: &mut ViewContext<Self>) -> String {
        let Some(text_input) = self.text_input.upgrade() else {
            return "".to_string();
        };

        let text_input = text_input.read(cx);

        let line_idx = text_input.content.char_to_line(text_input.cursor_offset());
        let line_char_idx = text_input.content.line_to_char(line_idx);
        let char_idx_in_line = text_input.cursor_offset() - line_char_idx;

        let selection = if !text_input.selected_range.is_empty() {
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
        } else {
            "".to_string()
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
            .text_size(px(14.))
            .child(
                div()
                    .px(px(4.))
                    .id("soft_wrap")
                    .on_click(cx.listener(Self::toggle_soft_wrap))
                    .child(self.soft_wrap_status(cx))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.rounded(px(6.)).bg(rgb(0x000000))),
            )
            .child(div().flex_grow())
            .child(self.selection_format(cx))
    }
}
