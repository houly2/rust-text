use crate::command::{Command, DeleteCommand, InsertCommand};
use crate::scroll_manager::ScrollManager;
use crate::settings_manager::CurrentSettings;
use crate::theme_manager::ActiveTheme;
use crate::{blink_manager::BlinkManager, lines::Lines, text_element::TextElement};

use gpui::*;
use ropey::*;
use std::cell::RefCell;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use unicode_segmentation::*;

actions!(
    text_input,
    [
        NewLine,
        NewLineWithoutSplit,
        Backspace,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
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
        MoveToDocStart,
        MoveToDocEnd,
        SelectWordStart,
        SelectWordEnd,
        SelectLineStart,
        SelectLineEnd,
        SelectDocStart,
        SelectDocEnd,
        Undo,
        Redo,
    ]
);

type PaintCallback = Box<dyn FnOnce(&mut TextInput, &mut ViewContext<TextInput>)>;

pub struct TextInput {
    pub focus_handle: FocusHandle,
    pub content: Rope,
    pub selected_range: Range<usize>,
    selection_reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<Lines>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub last_offset: Option<Point<Pixels>>,
    is_selecting: bool,
    is_scroll_dragging: bool,

    pub blink_manager: Model<BlinkManager>,
    pub scroll_manager: Model<ScrollManager>,

    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,

    on_next_paint_stack: Rc<RefCell<Vec<PaintCallback>>>,

    current_file_path: Option<PathBuf>,
    is_dirty: bool,

    settings_soft_wrap: bool,

    _subscriptions: Vec<Subscription>,
}

impl TextInput {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let blink_manager = cx.new_model(|_| BlinkManager::new());
        let scroll_manager = cx.new_model(|_| ScrollManager::new());


        cx.bind_keys([
            KeyBinding::new("enter", NewLine, None),
            KeyBinding::new("cmd-enter", NewLineWithoutSplit, None),
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("shift-up", SelectUp, None),
            KeyBinding::new("shift-down", SelectDown, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("alt-left", MoveToWordStart, None),
            KeyBinding::new("alt-right", MoveToWordEnd, None),
            KeyBinding::new("cmd-left", MoveToLineStart, None),
            KeyBinding::new("cmd-right", MoveToLineEnd, None),
            KeyBinding::new("cmd-up", MoveToDocStart, None),
            KeyBinding::new("cmd-down", MoveToDocEnd, None),
            KeyBinding::new("shift-alt-left", SelectWordStart, None),
            KeyBinding::new("shift-alt-right", SelectWordEnd, None),
            KeyBinding::new("shift-cmd-left", SelectLineStart, None),
            KeyBinding::new("shift-cmd-right", SelectLineEnd, None),
            KeyBinding::new("shift-cmd-up", SelectDocStart, None),
            KeyBinding::new("shift-cmd-down", SelectDocEnd, None),
            KeyBinding::new("cmd-z", Undo, None),
            KeyBinding::new("shift-cmd-z", Redo, None),
        ]);

        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            last_offset: None,
            is_selecting: false,
            is_scroll_dragging: false,
            blink_manager: blink_manager.clone(),
            scroll_manager: scroll_manager.clone(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            on_next_paint_stack: Default::default(),
            is_dirty: false,
            current_file_path: None,
            settings_soft_wrap: false,
            _subscriptions: vec![
                cx.observe(&scroll_manager, |_, _, cx| cx.notify()),
                cx.observe(&blink_manager, |_, _, cx| cx.notify()),
                cx.observe_window_activation(|this, cx| {
                    let active = cx.is_window_active();
                    if active {
                        this.scroll_manager.update(cx, |scroll_manager, cx| {
                            scroll_manager.enable(cx);
                        });
                    }
                    this.blink_manager.update(cx, |blink_manager, cx| {
                        if active {
                            blink_manager.enable(cx);
                        } else {
                            blink_manager.disable();
                        }
                    });
                }),
            ],
        }
    }

    pub fn notify_about_paint(&mut self, cx: &mut ViewContext<Self>) {
        let next_paint_callbacks = self.on_next_paint_stack.take();
        for callback in next_paint_callbacks {
            callback(self, cx);
        }
    }

    fn on_next_paint(
        &mut self,
        on_notify: impl FnOnce(&mut Self, &mut ViewContext<Self>) + 'static,
    ) {
        RefCell::borrow_mut(&self.on_next_paint_stack).push(Box::new(on_notify));
    }

    pub fn execute_command(&mut self, command: Box<dyn Command>, cx: &mut ViewContext<Self>) {
        command.execute(&mut self.content);
        self.undo_stack.push(command);
        self.redo_stack.clear();
        self.is_dirty = true;
        cx.notify();
    }

    pub fn undo(&mut self, _: &Undo, cx: &mut ViewContext<Self>) {
        if let Some(command) = self.undo_stack.pop() {
            let prev_selection = command.undo(&mut self.content);
            self.redo_stack.push(command);
            self.update_selected_range(prev_selection, cx);
        }
    }

    pub fn redo(&mut self, _: &Redo, cx: &mut ViewContext<Self>) {
        if let Some(command) = self.redo_stack.pop() {
            let new_selection = command.execute(&mut self.content);
            self.undo_stack.push(command);
            self.update_selected_range(new_selection, cx);
        }
    }

    pub fn insert(&mut self, text: String, cx: &mut ViewContext<Self>) {
        self.replace_text_in_range(None, &text, cx);
    }

    pub fn set_file_path(&mut self, path: PathBuf, cx: &mut ViewContext<Self>) {
        self.current_file_path = Some(path);
        cx.notify();
    }

    fn new_line(&mut self, _: &NewLine, cx: &mut ViewContext<Self>) {
        // todo: handle selection
        self.replace_text_in_range(None, "\n", cx);
    }

    fn new_line_without_split(&mut self, _: &NewLineWithoutSplit, cx: &mut ViewContext<Self>) {
        // todo: handle selection
        self.move_to(self.position_for_end_of_line(self.cursor_offset()), cx);
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
        if let Some(pos) = self.position_for_up() {
            self.move_to(pos, cx)
        }
    }

    fn down(&mut self, _: &Down, cx: &mut ViewContext<Self>) {
        if let Some(pos) = self.position_for_down() {
            self.move_to(pos, cx)
        }
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
        } else {
            let start_of_line_idx = self.position_for_start_of_line();
            let end_of_line_idx =
                self.next_boundary(self.position_for_end_of_line(self.cursor_offset()));
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content
                    .slice(start_of_line_idx..end_of_line_idx)
                    .to_string(),
            ));
        }
    }

    fn paste(&mut self, _: &Paste, cx: &mut ViewContext<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, cx);
        }
    }

    fn cut(&mut self, _: &Cut, cx: &mut ViewContext<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content.slice(self.selected_range.clone()).to_string(),
            ));
            self.replace_text_in_range(None, "", cx);
        } else {
            let start_of_line_idx = self.position_for_start_of_line();
            let end_of_line_idx =
                self.next_boundary(self.position_for_end_of_line(self.cursor_offset()));
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content
                    .slice(start_of_line_idx..end_of_line_idx)
                    .to_string(),
            ));
            self.replace_text_in_range(Some(start_of_line_idx..end_of_line_idx), "", cx);
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut ViewContext<Self>) {
        if let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        {
            if self
                .scroll_manager
                .read(cx)
                .is_in_scrollbar(event.position, bounds)
            {
                self.is_scroll_dragging = true;
                self.scroll_manager.update(cx, |this, cx| {
                    this.scroll_to(event.position, lines, bounds, cx)
                });
                return;
            }
        }

        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else if event.click_count == 2 {
            let offset = self.index_for_mouse_position(event.position);
            let prev = self.start_of_word(self.next_boundary(offset));
            let next = self.end_of_word(offset);
            self.move_to(prev, cx);
            self.select_to(next, cx);
        } else if event.click_count == 3 {
            let start_of_line_idx = self.position_for_start_of_line();
            self.move_to(start_of_line_idx, cx);
            let end_of_line_idx = self.position_for_end_of_line(self.cursor_offset());
            self.select_to(end_of_line_idx, cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut ViewContext<Self>) {
        self.is_selecting = false;
        self.is_scroll_dragging = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut ViewContext<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else if self.is_scroll_dragging {
            if let (Some(bounds), Some(lines)) =
                (self.last_bounds.as_ref(), self.last_layout.as_ref())
            {
                self.scroll_manager.update(cx, |this, cx| {
                    this.scroll_to(event.position, lines, bounds, cx)
                });
            }
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.len_chars() == 0 {
            return 0;
        }

        let (Some(bounds), Some(lines), Some(offset)) = (
            self.last_bounds.as_ref(),
            self.last_layout.as_ref(),
            self.last_offset.as_ref(),
        ) else {
            return 0;
        };

        let offset_position = point(position.x - offset.x, position.y + offset.y.abs());

        if position.y < bounds.top() {
            return 0;
        }

        if position.y > bounds.bottom() || offset_position.y > bounds.origin.y + lines.height() {
            return self.content.len_chars();
        }

        let pos = point(
            offset_position.x - bounds.left(),
            offset_position.y - bounds.top(),
        );

        if let Some((line_idx, byte_idx)) = lines.byte_index_for_position(pos) {
            let line = self.content.line_to_byte(line_idx);
            return self.content.byte_to_char(line + byte_idx);
        }

        if let Some(line_idx) = lines.line_idx_for_y(pos.y) {
            let line = self.content.line_to_char(line_idx);
            return self.position_for_end_of_line(line);
        }

        return 0;
    }

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut ViewContext<Self>) {
        if let (Some(lines), Some(bounds)) = (self.last_layout.as_ref(), self.last_bounds.as_ref())
        {
            self.scroll_manager.update(cx, |this, cx| {
                this.calc_offset_after_scroll(event.delta, lines, bounds, cx)
            })
        };
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

    fn select_up(&mut self, _: &SelectUp, cx: &mut ViewContext<Self>) {
        if let Some(pos) = self.position_for_up() {
            self.select_to(pos, cx);
        }
    }

    fn select_down(&mut self, _: &SelectDown, cx: &mut ViewContext<Self>) {
        if let Some(pos) = self.position_for_down() {
            self.select_to(pos, cx);
        }
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
        let start_of_line_idx = self.position_for_start_of_line();
        self.select_to(start_of_line_idx, cx)
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, cx: &mut ViewContext<Self>) {
        self.select_to(self.position_for_end_of_line(self.cursor_offset()), cx);
    }

    fn move_to_word_start(&mut self, _: &MoveToWordStart, cx: &mut ViewContext<Self>) {
        self.move_to(self.start_of_word(self.cursor_offset()), cx)
    }

    fn move_to_word_end(&mut self, _: &MoveToWordEnd, cx: &mut ViewContext<Self>) {
        self.move_to(self.end_of_word(self.cursor_offset()), cx)
    }

    fn move_to_line_start(&mut self, _: &MoveToLineStart, cx: &mut ViewContext<Self>) {
        let start_of_line_idx = self.position_for_start_of_line();
        self.move_to(start_of_line_idx, cx)
    }

    fn move_to_line_end(&mut self, _: &MoveToLineEnd, cx: &mut ViewContext<Self>) {
        self.move_to(self.position_for_end_of_line(self.cursor_offset()), cx)
    }

    fn move_to_doc_start(&mut self, _: &MoveToDocStart, cx: &mut ViewContext<Self>) {
        self.move_to(0, cx);
    }

    fn move_to_doc_end(&mut self, _: &MoveToDocEnd, cx: &mut ViewContext<Self>) {
        self.move_to(self.content.len_chars(), cx);
    }

    fn select_doc_start(&mut self, _: &SelectDocStart, cx: &mut ViewContext<Self>) {
        self.select_to(0, cx);
    }

    fn select_doc_end(&mut self, _: &SelectDocEnd, cx: &mut ViewContext<Self>) {
        self.select_to(self.content.len_chars(), cx);
    }

    pub fn move_to(&mut self, offset: usize, cx: &mut ViewContext<Self>) {
        self.selected_range = offset..offset;
        self.blink_manager.update(cx, BlinkManager::pause);

        let epoch = self
            .scroll_manager
            .update(cx, |this, _| this.next_calc_epoch());
        self.update_scroll_manager(epoch, offset, cx);
        cx.notify();
    }

    fn update_scroll_manager(&mut self, epoch: usize, offset: usize, cx: &mut ViewContext<Self>) {
        if let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        {
            let line_idx = self.content.char_to_line(offset);
            let byte_idx = self.content.line_to_byte(line_idx);
            let cursor_idx = self.content.char_to_byte(offset);

            self.scroll_manager.update(cx, |this, cx| {
                this.calc_offset_after_move(
                    epoch,
                    line_idx,
                    cursor_idx - byte_idx,
                    lines,
                    bounds,
                    cx,
                )
            });
        }
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

        let epoch = self
            .scroll_manager
            .update(cx, |this, _| this.next_calc_epoch());
        self.update_scroll_manager(epoch, offset, cx);
        cx.notify();
    }

    // - Helper

    fn position_from_layout(&mut self, position: usize) -> Option<Point<Pixels>> {
        if let Some(layout) = &self.last_layout {
            let line_idx = self.content.char_to_line(position);
            let char_idx = self.content.line_to_byte(line_idx);
            let cursor_idx = self.content.char_to_byte(position);
            return Some(layout.position_for_byte_idx_in_line(cursor_idx - char_idx, line_idx));
        }
        None
    }

    fn position_for_up(&mut self) -> Option<usize> {
        if let (Some(cursor_pos), Some(layout)) = (
            self.position_from_layout(self.cursor_offset()),
            &self.last_layout,
        ) {
            let y = cursor_pos.y - layout.line_height;

            if y < px(0.) {
                return None;
            }

            let prev_visual_line_pos = point(cursor_pos.x, y);
            if let Some((line_idx, pos)) = layout.byte_index_for_position(prev_visual_line_pos) {
                let line = self.content.line_to_byte(line_idx);
                let char = self.content.byte_to_char(line + pos);
                return Some(char);
            } else {
                let line_idx = self.content.char_to_line(self.cursor_offset());
                let line_start = self.content.line_to_char(line_idx);
                return Some(self.previous_boundary(line_start));
            }
        }

        None
    }

    fn position_for_down(&mut self) -> Option<usize> {
        if let (Some(cursor_pos), Some(layout)) = (
            self.position_from_layout(self.cursor_offset()),
            &self.last_layout,
        ) {
            let next_visual_line_pos = point(cursor_pos.x, cursor_pos.y + layout.line_height);
            if let Some((line_idx, pos)) = layout.byte_index_for_position(next_visual_line_pos) {
                let line = self.content.line_to_byte(line_idx);
                return Some(self.content.byte_to_char(line + pos));
            } else {
                // did this line wrap?
                let end_of_line = self.position_for_end_of_line(self.cursor_offset());
                if let Some(end_pos) = self.position_from_layout(end_of_line) {
                    if cursor_pos.y < end_pos.y {
                        return Some(end_of_line);
                    }
                }

                // go to end of next line?
                let current_line = self.content.char_to_line(self.cursor_offset());
                if current_line < self.content.len_lines() {
                    let start_of_next_line = self.content.line_to_char(current_line + 1);
                    return Some(self.position_for_end_of_line(start_of_next_line));
                }
            }
        }

        None
    }

    fn position_for_start_of_line(&mut self) -> usize {
        if let (Some(cursor_pos), Some(layout)) = (
            self.position_from_layout(self.cursor_offset()),
            &self.last_layout,
        ) {
            let start_of_line_pos = point(px(0.), cursor_pos.y);
            if let Some((line_idx, pos)) = layout.byte_index_for_position(start_of_line_pos) {
                let line = self.content.line_to_byte(line_idx);
                return self.content.byte_to_char(line + pos);
            }
        }

        let current_line_idx = self.content.char_to_line(self.cursor_offset());
        self.content.line_to_char(current_line_idx)
    }

    fn position_for_end_of_line(&self, position: usize) -> usize {
        if let Some(layout) = &self.last_layout {
            let line_idx = self.content.char_to_line(position);
            let line_byte_idx = self.content.line_to_byte(line_idx);
            let pos_byte_idx_in_line = self.content.char_to_byte(position) - line_byte_idx;

            if let Some(end_byte_idx) = layout.wrapped_line_end_idx(line_idx, pos_byte_idx_in_line)
            {
                return self.content.byte_to_char(line_byte_idx + end_byte_idx);
            }
        }

        let line_idx = self.content.char_to_line(position);
        let line_char_idx = self.content.line_to_char(line_idx);
        line_char_idx + self.content.line(line_idx).len_chars()
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

    fn update_selected_range(&mut self, range: Range<usize>, cx: &mut ViewContext<Self>) {
        self.selected_range = range;
        self.marked_range.take();
        self.blink_manager.update(cx, BlinkManager::pause);
        cx.notify();
    }

    pub fn set_soft_wrap(&mut self, enabled: bool, cx: &mut ViewContext<Self>) {
        self.settings_soft_wrap = enabled;
        cx.notify();
    }

    pub fn soft_wrap_enabled(&self) -> bool {
        self.settings_soft_wrap
    }

    pub fn mark_dirty(&mut self, value: bool, cx: &mut ViewContext<Self>) {
        self.is_dirty = value;
        cx.notify();
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn file_path(&self) -> &Option<PathBuf> {
        &self.current_file_path
    }
}

impl Render for TextInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .text_color(cx.theme().editor_text)
            .font_family(cx.settings().font_family)
            .child(
                div()
                    .flex()
                    .h_full()
                    .w_full()
                    .overflow_hidden()
                    .bg(cx.theme().editor_background)
                    .line_height(px(28.))
                    .text_size(px(18.))
                    .key_context("TextInput")
                    .track_focus(&self.focus_handle(cx))
                    .cursor(CursorStyle::IBeam)
                    .on_action(cx.listener(Self::new_line))
                    .on_action(cx.listener(Self::new_line_without_split))
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
                    .on_action(cx.listener(Self::select_up))
                    .on_action(cx.listener(Self::select_down))
                    .on_action(cx.listener(Self::select_all))
                    .on_action(cx.listener(Self::show_character_palette))
                    .on_action(cx.listener(Self::copy))
                    .on_action(cx.listener(Self::paste))
                    .on_action(cx.listener(Self::cut))
                    .on_action(cx.listener(Self::move_to_word_start))
                    .on_action(cx.listener(Self::move_to_word_end))
                    .on_action(cx.listener(Self::move_to_line_start))
                    .on_action(cx.listener(Self::move_to_line_end))
                    .on_action(cx.listener(Self::move_to_doc_start))
                    .on_action(cx.listener(Self::move_to_doc_end))
                    .on_action(cx.listener(Self::select_word_start))
                    .on_action(cx.listener(Self::select_word_end))
                    .on_action(cx.listener(Self::select_line_start))
                    .on_action(cx.listener(Self::select_line_end))
                    .on_action(cx.listener(Self::select_doc_start))
                    .on_action(cx.listener(Self::select_doc_end))
                    .on_action(cx.listener(Self::undo))
                    .on_action(cx.listener(Self::redo))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
                    .on_mouse_move(cx.listener(Self::on_mouse_move))
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
        Some(self.content.slice(range_utf16).to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut ViewContext<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.selected_range.clone(),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut ViewContext<Self>) -> Option<std::ops::Range<usize>> {
        self.marked_range.clone()
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

        if range.start != range.end {
            let old_text = self.content.slice(range.clone()).to_string();
            self.execute_command(
                Box::new(DeleteCommand::new(
                    range.start,
                    old_text,
                    self.selected_range.clone(),
                )),
                cx,
            );
        }

        if !text.is_empty() {
            self.execute_command(
                Box::new(InsertCommand::new(
                    range.start,
                    text.to_string(),
                    self.selected_range.clone(),
                )),
                cx,
            );
        }

        let l = text.chars().count();
        self.update_selected_range(range.start + l..range.start + l, cx);
        let epoch = self
            .scroll_manager
            .update(cx, |this, _| this.next_calc_epoch());
        self.on_next_paint(move |this, cx| this.update_scroll_manager(epoch, range.start + l, cx));
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<std::ops::Range<usize>>,
        cx: &mut ViewContext<Self>,
    ) {
        if let Some(marked_range) = self.marked_range.take() {
            self.replace_text_in_range(Some(marked_range), "", cx);
        } else {
            let range = range_utf16.unwrap_or(self.selected_range.clone());
            self.content.remove(range.start..range.end);
            self.content.insert(range.start, new_text);

            let l = new_text.chars().count();
            self.marked_range = Some(range.start..range.start + l);
            self.selected_range = new_selected_range_utf16
                .as_ref()
                .map(|new_range| new_range.start + range.start..new_range.end + range.end)
                .unwrap_or_else(|| range.start + l..range.start + l);
        }

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
    }
}
