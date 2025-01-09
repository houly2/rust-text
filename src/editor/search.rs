use std::ops::Range;

use aho_corasick::AhoCorasickBuilder;
use gpui::*;

use crate::views::text_input::text_input::{NewLine, TextInput, TextInputMode};

actions!(search, [Close]);

pub struct SearchView {
    show: bool,
    view: View<TextInput>,
    text_input: WeakView<TextInput>,
    last_term: Option<String>,
    last_highlight_idx: Option<usize>,
    last_matches: Option<Vec<Range<usize>>>,
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
                    let new_idx = if last_idx < last_matches.len() - 1 {
                        last_idx + 1
                    } else {
                        0
                    };
                    self.last_highlight_idx = Some(new_idx);
                    let next_match = last_matches.get(new_idx).unwrap();
                    text_input.update(cx, |text_input, cx| {
                        text_input.update_selected_range_bytes(next_match, cx);
                    });
                }
            }
            return;
        }

        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
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
            text_input.update_selected_range_bytes(matches.first().unwrap(), cx);
        });
    }

    fn close(&mut self, _: &Close, cx: &mut ViewContext<Self>) {
        self.hide(cx);
    }
}

impl Render for SearchView {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl IntoElement {
        if !self.show {
            return div();
        }

        div()
            .flex()
            .flex_row()
            .key_context("search")
            .on_action(cx.listener(Self::close))
            .child(self.view.clone())
    }
}
