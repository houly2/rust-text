use std::ops::Range;

use aho_corasick::AhoCorasickBuilder;
use gpui::*;

use crate::{
    settings_manager::CurrentSettings,
    theme_manager::ActiveTheme,
    views::{
        icons::Icons,
        text_input::text_input::{NewLine, TextInput, TextInputMode},
    },
};

actions!(search, [Close]);

pub struct SearchView {
    show: bool,
    view: View<TextInput>,
    text_input: WeakView<TextInput>,
    last_term: Option<String>,
    last_highlight_idx: Option<usize>,
    last_matches: Option<Vec<Range<usize>>>,
    case_insensitive: bool,
    _subscriptions: Vec<Subscription>,
}

impl SearchView {
    pub fn new(text_input: WeakView<TextInput>, cx: &mut ViewContext<Self>) -> Self {
        let search_view = cx.new_view(|cx| TextInput::new(TextInputMode::SingleLine, cx));

        cx.bind_keys([KeyBinding::new("escape", Close, None)]);

        Self {
            show: false,
            view: search_view.clone(),
            text_input,
            last_term: None,
            last_highlight_idx: None,
            last_matches: None,
            case_insensitive: true,
            _subscriptions: vec![
                cx.subscribe(&search_view, |this, _, _: &NewLine, cx| this.search(cx))
            ],
        }
    }

    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.show = true;
        cx.focus_view(&self.view);
        self.view.update(cx, |t, cx| t.select_all(cx));
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        self.show = false;
        if let Some(text_input) = self.text_input.upgrade() {
            text_input.update(cx, |text_input, cx| text_input.clear_highlights(cx));
            cx.focus_view(&text_input);
        }
        cx.notify();
    }

    fn search(&mut self, cx: &mut ViewContext<Self>) {
        let Some(text_input) = self.text_input.upgrade() else {
            return;
        };

        let term = &self.view.read(cx).content.to_string();

        if term.is_empty() {
            return;
        }

        // this can't be right?
        if self.last_term.clone().is_some_and(|t| t == *term) {
            if let Some(last_idx) = self.last_highlight_idx {
                if let Some(last_matches) = &self.last_matches {
                    let new_idx = if !last_matches.is_empty() && last_idx < last_matches.len() - 1 {
                        last_idx + 1
                    } else {
                        0
                    };
                    self.last_highlight_idx = Some(new_idx);
                    if let Some(next_match) = last_matches.get(new_idx) {
                        text_input.update(cx, |text_input, cx| {
                            text_input.highlight(last_matches.clone(), cx);
                            text_input.update_selected_range_bytes(next_match, cx);
                        });
                    }
                }
            }
            return;
        }

        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(self.case_insensitive)
            .build([term])
            .unwrap();

        let haystack = text_input.read(cx).content.to_string();

        let mut matches = vec![];
        for mat in ac.find_iter(&haystack) {
            matches.push(mat.span().range());
        }

        self.last_term = Some(term.to_string());
        self.last_highlight_idx = Some(0);
        self.last_matches = Some(matches.clone());

        text_input.update(cx, |text_input, cx| {
            text_input.highlight(matches.clone(), cx);
            if let Some(first) = matches.first() {
                text_input.update_selected_range_bytes(first, cx);
            }
        });
    }

    fn close(&mut self, _: &Close, cx: &mut ViewContext<Self>) {
        self.hide(cx);
    }

    fn close_handler(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.hide(cx);
    }

    fn toggle_case(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.case_insensitive = !self.case_insensitive;
        self.reset();
        self.search(cx);
    }

    fn reset(&mut self) {
        self.last_term = None;
        self.last_highlight_idx = None;
        self.last_matches = None;
    }
}

impl Render for SearchView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        if !self.show {
            return div();
        }

        let has_error = if let Some(matches) = &self.last_matches {
            matches.is_empty()
        } else {
            false
        };

        div()
            .flex()
            .flex_row()
            .key_context("search")
            .on_action(cx.listener(Self::close))
            .bg(cx.theme().background)
            .line_height(px(28.))
            .text_size(px(18.))
            .text_color(if has_error {
                cx.theme().error
            } else {
                cx.theme().editor_text
            })
            .font_family(cx.settings().font_family)
            .child(self.view.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .px(px(4.))
                    .items_center()
                    .child(
                        div()
                            .id("toggle_case")
                            .on_click(cx.listener(Self::toggle_case))
                            .child(Icons::CharacterSentenceCase.as_button(!self.case_insensitive)),
                    )
                    .child(
                        div()
                            .id("close")
                            .on_click(cx.listener(Self::close_handler))
                            .child(Icons::Close.as_button(false)),
                    ),
            )
    }
}
