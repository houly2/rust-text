use crate::lines::Lines;
use crate::text_input::TextInput;
use gpui::*;
use smallvec::SmallVec;

pub struct TextElement {
    input: View<TextInput>,
}

impl TextElement {
    pub fn new(input: View<TextInput>) -> Self {
        Self { input }
    }

    fn get_line_selection_bounds(
        &self,
        bounds: &Bounds<Pixels>,
        start: Point<Pixels>,
        end: Point<Pixels>,
        line_height: Pixels,
    ) -> Bounds<Pixels> {
        Bounds::from_corners(
            point(bounds.left() + start.x, bounds.top() + start.y),
            point(bounds.left() + end.x, bounds.top() + start.y + line_height),
        )
    }
}

pub struct PrepaintState {
    offset: Point<Pixels>,
    bounds: Bounds<Pixels>,
    lines: Option<Lines>,
    cursor: Option<PaintQuad>,
    selections: Option<Vec<PaintQuad>>,
    scroll_bar: Option<SmallVec<[PaintQuad; 2]>>,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (cx.request_layout(style, []), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = cx.text_style();
        let text_color = style.color;

        let padding = px(8.);
        let new_bounds = Bounds::new(
            point(bounds.origin.x + padding, bounds.origin.y + padding),
            size(
                bounds.size.width - padding * 2,
                bounds.size.height - padding * 2,
            ),
        );

        let run = TextRun {
            len: input.content.len_bytes(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: input.content.len_bytes() - marked_range.end,
                    ..run.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(cx.rem_size());
        let display_text = input.content.clone();
        let text: SharedString = display_text.to_string().into();

        let lines_raw = cx
            .text_system()
            .shape_text(text.clone(), font_size, &runs, None)
            .unwrap();

        let line_height = cx.line_height();
        let lines = Lines::new(lines_raw, line_height);

        let selections: Option<Vec<PaintQuad>>;
        let paint_cursor: Option<PaintQuad>;
        let scroll_bar: Option<SmallVec<[PaintQuad; 2]>>;

        let line_idx = display_text.char_to_line(cursor);
        let char_idx = display_text.line_to_byte(line_idx);
        let cursor_idx = display_text.char_to_byte(cursor);
        let cursor_pos = lines.position_for_index_in_line(cursor_idx - char_idx, line_idx);

        let scroll_manager = input.scroll_manager.read(cx);
        scroll_bar = scroll_manager.paint_bar(bounds, lines.height());
        let offset = scroll_manager.offset;

        if input.blink_manager.read(cx).show() {
            paint_cursor = Some(fill(
                Bounds::new(
                    point(
                        new_bounds.left() + cursor_pos.x,
                        new_bounds.top() + cursor_pos.y,
                    ),
                    size(px(2.), line_height),
                ),
                rgb(0xcdd6f4),
            ))
        } else {
            paint_cursor = None;
        }

        if selected_range.is_empty() {
            selections = None;
        } else {
            let start = display_text.char_to_byte(selected_range.start);
            let start_line_idx = display_text.byte_to_line(start);
            let start_char_idx = display_text.line_to_byte(start_line_idx);
            let start_point =
                lines.position_for_index_in_line(start - start_char_idx, start_line_idx);

            let end = display_text.char_to_byte(selected_range.end);
            let end_line_idx = display_text.byte_to_line(end);
            let end_char_idx = display_text.line_to_byte(end_line_idx);
            let end_point = lines.position_for_index_in_line(end - end_char_idx, end_line_idx);

            selections = {
                let selection_color = rgba(0x7f849c64);
                let line_count: u32 =
                    ((end_point.y - start_point.y) / lines.line_height).round() as u32;
                let mut selection_quads = Vec::new();

                match line_count {
                    0 => {
                        // Single-line selection
                        let bounds = self.get_line_selection_bounds(
                            &new_bounds,
                            start_point,
                            end_point,
                            line_height,
                        );
                        selection_quads.push(fill(bounds, selection_color));
                    }
                    1 => {
                        // Two-line selection
                        let start_line = lines.line(start_line_idx).unwrap();
                        let first_line_end =
                            point(start_line.size(lines.line_height).width, start_point.y);
                        let second_line_start = point(px(0.), end_point.y);

                        selection_quads.push(fill(
                            self.get_line_selection_bounds(
                                &new_bounds,
                                start_point,
                                first_line_end,
                                line_height,
                            ),
                            selection_color,
                        ));
                        selection_quads.push(fill(
                            self.get_line_selection_bounds(
                                &new_bounds,
                                second_line_start,
                                end_point,
                                line_height,
                            ),
                            selection_color,
                        ));
                    }
                    _ => {
                        // Multi-line selection
                        let start_line = lines.line(start_line_idx).unwrap();

                        // First line
                        let first_line_end =
                            point(start_line.size(lines.line_height).width, start_point.y);
                        selection_quads.push(fill(
                            self.get_line_selection_bounds(
                                &new_bounds,
                                start_point,
                                first_line_end,
                                line_height,
                            ),
                            selection_color,
                        ));

                        // Middle lines
                        for n in 1..line_count {
                            let line = lines.line(start_line_idx + n as usize).unwrap();
                            let line_y = start_point.y + px(n as f32) * lines.line_height;
                            let line_bounds = self.get_line_selection_bounds(
                                &new_bounds,
                                point(px(0.), line_y),
                                point(line.size(lines.line_height).width, line_y),
                                line_height,
                            );
                            selection_quads.push(fill(line_bounds, selection_color));
                        }

                        // Last line
                        let last_line_bounds = self.get_line_selection_bounds(
                            &new_bounds,
                            point(px(0.), end_point.y),
                            end_point,
                            line_height,
                        );
                        selection_quads.push(fill(last_line_bounds, selection_color));
                    }
                }
                Some(selection_quads)
            }
        }

        PrepaintState {
            offset,
            bounds: new_bounds,
            lines: Some(lines),
            cursor: paint_cursor,
            selections,
            scroll_bar,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        cx.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
        );

        if let Some(selections) = prepaint.selections.take() {
            for selection in selections {
                let mut selection = selection.clone();
                selection.bounds.origin.x = prepaint.offset.x + selection.bounds.origin.x;
                selection.bounds.origin.y = prepaint.offset.y + selection.bounds.origin.y;
                cx.paint_quad(selection);
            }
        }

        let line_height = cx.line_height();
        let mut offset_y = prepaint.offset.y;
        let lines = prepaint.lines.take().unwrap();
        for line in &lines.lines {
            let size = line.size(line_height);
            line.paint(
                point(
                    prepaint.bounds.origin.x + prepaint.offset.x,
                    prepaint.bounds.origin.y + offset_y,
                ),
                line_height,
                cx,
            )
            .unwrap();
            offset_y += size.height;
        }

        if let Some(cursor) = prepaint.cursor.take() {
            let mut cursor = cursor.clone();
            cursor.bounds.origin.x = prepaint.offset.x + cursor.bounds.origin.x;
            cursor.bounds.origin.y = prepaint.offset.y + cursor.bounds.origin.y;
            cx.paint_quad(cursor);
        }

        if let Some(scroll_bar) = prepaint.scroll_bar.take() {
            for bar in scroll_bar {
                cx.paint_quad(bar);
            }
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(lines);
            input.last_bounds = Some(prepaint.bounds);
            input.last_offset = Some(prepaint.offset);
        });
    }
}
