use crate::{blink_manager::BlinkManager, text_element::Lines, text_element::TextElement};

use gpui::*;
use ropey::*;
use std::ops::Range;
use unicode_segmentation::*;

actions!(
    text_input,
    [
        NewLine,
        Backspace,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectAll,
        Delete,
        Home,
        End,
        ShowCharacterPalette,
        Copy,
        Paste,
        Cut,
        MoveToWordStart,
        MoveToWordEnd,
        MoveToLineStart,
        MoveToLineEnd,
        SelectWordStart,
        SelectWordEnd,
        SelectLineStart,
        SelectLineEnd,
    ]
);

pub struct TextInput {
    pub focus_handle: FocusHandle,
    pub content: Rope,
    pub selected_range: Range<usize>,
    selection_reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<Lines>,
    pub last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,

    pub blink_manager: Model<BlinkManager>,

    _subscriptions: Vec<Subscription>,
}

impl TextInput {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let blink_manager = cx.new_model(|_| BlinkManager::new());

        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            blink_manager: blink_manager.clone(),
            _subscriptions: vec![
                cx.observe(&blink_manager, |_, _, cx| cx.notify()),
                cx.observe_window_activation(|this, cx| {
                    let active = cx.is_window_active();
                    this.blink_manager.update(cx, |blink_manager, cx| {
                        if active {
                            blink_manager.enable(cx);
                        } else {
                            blink_manager.disable(cx);
                        }
                    });
                }),
            ],
        }
    }

    fn new_line(&mut self, _: &NewLine, cx: &mut ViewContext<Self>) {
        // todo: handle selection
        self.replace_text_in_range(None, "\n", cx);
    }

    fn backspace(&mut self, _: &Backspace, cx: &mut ViewContext<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", cx);
    }

    fn delete(&mut self, _: &Delete, cx: &mut ViewContext<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", cx);
    }

    fn left(&mut self, _: &Left, cx: &mut ViewContext<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, cx: &mut ViewContext<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn up(&mut self, _: &Up, cx: &mut ViewContext<Self>) {
        let current = self.cursor_offset();
        let line_idx = self.content.char_to_line(current);

        if line_idx == 0 {
            return;
        }

        let this_line = self.content.line_to_char(line_idx);
        let prev_line = self.content.line_to_char(line_idx - 1);
        let line_offset = std::cmp::min(
            current - this_line,
            self.content.line(line_idx - 1).len_chars() - 1,
        );

        self.move_to(prev_line + line_offset, cx)
    }

    fn down(&mut self, _: &Down, cx: &mut ViewContext<Self>) {
        let current = self.cursor_offset();
        let line_idx = self.content.char_to_line(current);

        if line_idx == self.content.len_lines() - 1 {
            return;
        }

        let this_line = self.content.line_to_char(line_idx);
        let next_line = self.content.line_to_char(line_idx + 1);
        let line_offset = std::cmp::min(
            current - this_line,
            self.content.line(line_idx + 1).len_chars(),
        );

        self.move_to(next_line + line_offset, cx)
    }

    fn home(&mut self, _: &Home, cx: &mut ViewContext<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, cx: &mut ViewContext<Self>) {
        self.move_to(self.content.len_chars(), cx);
    }

    fn copy(&mut self, _: &Copy, cx: &mut ViewContext<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content.slice(self.selected_range.clone()).to_string(),
            ));
        }
    }

    fn paste(&mut self, _: &Paste, cx: &mut ViewContext<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), cx);
        }
    }

    fn cut(&mut self, _: &Cut, cx: &mut ViewContext<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content.slice(self.selected_range.clone()).to_string(),
            ));
            self.replace_text_in_range(None, "", cx);
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut ViewContext<Self>) {
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else if event.click_count == 2 {
            let offset = self.index_for_mouse_position(event.position);
            let prev = self.start_of_word(offset);
            let next = self.end_of_word(offset);
            self.move_to(next, cx);
            self.select_to(prev, cx);
        } else if event.click_count == 3 {
            self.move_to(self.content.len_chars(), cx);
            self.select_to(0, cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut ViewContext<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut ViewContext<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.len_chars() == 0 {
            return 0;
        }

        let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len_chars();
        }

        if let Some(idx) =
            lines.index_for_position(point(position.x - bounds.left(), position.y - bounds.top()))
        {
            return self.content.byte_to_char(idx);
        }

        return 0;
    }

    fn show_character_palette(&mut self, _: &ShowCharacterPalette, cx: &mut ViewContext<Self>) {
        cx.show_character_palette();
    }

    fn select_left(&mut self, _: &SelectLeft, cx: &mut ViewContext<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, cx: &mut ViewContext<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, cx: &mut ViewContext<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len_chars(), cx);
    }

    fn select_word_start(&mut self, _: &SelectWordStart, cx: &mut ViewContext<Self>) {
        self.select_to(self.start_of_word(self.cursor_offset()), cx);
    }

    fn select_word_end(&mut self, _: &SelectWordEnd, cx: &mut ViewContext<Self>) {
        self.select_to(self.end_of_word(self.cursor_offset()), cx);
    }

    fn select_line_start(&mut self, _: &SelectLineStart, cx: &mut ViewContext<Self>) {
        self.select_to(0, cx);
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, cx: &mut ViewContext<Self>) {
        self.select_to(self.content.len_chars(), cx);
    }

    fn move_to_word_start(&mut self, _: &MoveToWordStart, cx: &mut ViewContext<Self>) {
        self.move_to(self.start_of_word(self.cursor_offset()), cx)
    }

    fn move_to_word_end(&mut self, _: &MoveToWordEnd, cx: &mut ViewContext<Self>) {
        self.move_to(self.end_of_word(self.cursor_offset()), cx)
    }

    fn move_to_line_start(&mut self, _: &MoveToLineStart, cx: &mut ViewContext<Self>) {
        self.move_to(0, cx)
    }

    fn move_to_line_end(&mut self, _: &MoveToLineEnd, cx: &mut ViewContext<Self>) {
        self.move_to(self.content.len_chars(), cx)
    }

    fn move_to(&mut self, offset: usize, cx: &mut ViewContext<Self>) {
        self.selected_range = offset..offset;
        self.blink_manager.update(cx, BlinkManager::pause);
        cx.notify();
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut ViewContext<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset;
        }

        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }

        cx.notify();
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn start_of_word(&self, offset: usize) -> usize {
        let c = self.content.char_to_byte(self.previous_boundary(offset));
        let t = self
            .content
            .to_string()
            .unicode_word_indices()
            .rev()
            .find_map(|(idx, _)| (idx < c).then_some(idx))
            .unwrap_or(0);

        return self.content.byte_to_char(t);
    }

    fn end_of_word(&self, offset: usize) -> usize {
        let mut skip = 0;
        for charr in self.content.chars_at(offset) {
            if charr != ' ' {
                break;
            }
            skip += 1;
        }

        let c = self.content.char_to_byte(offset + skip);

        let t = self
            .content
            .to_string()
            .unicode_word_indices()
            .rev()
            .find_map(|(idx, word)| (idx <= c).then_some(idx + word.len()))
            .unwrap_or(0);

        return self.content.byte_to_char(t);
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        if offset > 0 {
            offset - 1
        } else {
            0
        }
    }

    fn next_boundary(&self, offset: usize) -> usize {
        if offset < self.content.len_chars() {
            offset + 1
        } else {
            self.content.len_chars()
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .h_full()
            .w_full()
            .key_context("TextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::new_line))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::move_to_word_start))
            .on_action(cx.listener(Self::move_to_word_end))
            .on_action(cx.listener(Self::move_to_line_start))
            .on_action(cx.listener(Self::move_to_line_end))
            .on_action(cx.listener(Self::select_word_start))
            .on_action(cx.listener(Self::select_word_end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(
                div()
                    .flex()
                    .h_full()
                    .w_full()
                    .p(px(4.))
                    .bg(rgb(0x1e1e2e))
                    .line_height(px(24.))
                    .text_size(px(18.))
                    .text_color(rgb(0xcdd6f4))
                    .font_family("Iosevka")
                    .child(TextElement::new(cx.view().clone())),
            )
    }
}

impl FocusableView for TextInput {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ViewInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _: &mut ViewContext<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content.slice(range).to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut ViewContext<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut ViewContext<Self>) -> Option<std::ops::Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut ViewContext<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        cx: &mut ViewContext<Self>,
    ) {
        let range = range_utf16
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content.remove(range.start..range.end);
        self.content.insert(range.start, text);

        let l = text.chars().count();
        self.selected_range = range.start + l..range.start + l;
        self.marked_range.take();

        self.blink_manager.update(cx, BlinkManager::pause);

        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<std::ops::Range<usize>>,
        cx: &mut ViewContext<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content.remove(range.start..range.end);
        self.content.insert(range.start, new_text);

        let l = new_text.chars().count();
        self.marked_range = Some(range.start..range.start + l);
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + l..range.start + l);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: std::ops::Range<usize>,
        _: Bounds<Pixels>,
        _: &mut ViewContext<Self>,
    ) -> Option<Bounds<Pixels>> {
        println!("bounds_for_range");

        None
        // let last_layout = self.last_layout.as_ref()?;
        // let range = self.range_from_utf16(&range_utf16);

        // Some(Bounds::from_corners(
        //     point(
        //         bounds.left() + last_layout.x_for_index(range.start),
        //         bounds.top(),
        //     ),
        //     point(
        //         bounds.left() + last_layout.x_for_index(range.end),
        //         bounds.bottom(),
        //     ),
        // ))
    }
}
