use aho_corasick::AhoCorasickBuilder;
use gpui::*;

use crate::{NewLine, TextInput, TextInputMode};

actions!(search, [Close]);

pub struct SearchView {
    show: bool,
    view: View<TextInput>,
    text_input: WeakView<TextInput>,
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
            _subscriptions: vec![
                cx.subscribe(&search_view, |this, _, _: &NewLine, cx| this.search(cx))
            ],
        }
    }

    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.show = true;
        cx.focus_view(&self.view);
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

        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .build([term])
            .unwrap();

        let haystack = text_input.read(cx).content.to_string();

        let mut matches = vec![];
        for mat in ac.find_iter(&haystack) {
            matches.push(mat.span().range());
        }

        text_input.update(cx, |text_input, cx| text_input.highlight(matches, cx));
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
            .key_context("search")
            .on_action(cx.listener(Self::close))
            .child(self.view.clone())
    }
}
