use crate::text_input::TextInput;
use gpui::*;
use smallvec::SmallVec;

pub struct Lines {
    lines: SmallVec<[WrappedLine; 1]>,
    line_height: Pixels,
}

impl Lines {
    pub fn new(lines: SmallVec<[WrappedLine; 1]>, line_height: Pixels) -> Self {
        Self { lines, line_height }
    }
}

impl Lines {
    pub fn position_for_index_in_line(
        &self,
        index: usize,
        line_number: usize,
    ) -> Option<Point<Pixels>> {
        if let Some(line) = self.lines.get(line_number) {
            if let Some(a) = line.position_for_index(index, self.line_height) {
                return Some(point(a.x, a.y + px(line_number as f32) * self.line_height));
            }
        }
        None
    }

    pub fn index_for_position(&self, position: Point<Pixels>) -> Option<usize> {
        for line in &self.lines {
            match line.index_for_position(position, self.line_height) {
                Ok(v) => return Some(v),
                _ => continue,
            }
        }

        None
    }
}

pub struct TextElement {
    input: View<TextInput>,
}

impl TextElement {
    pub fn new(input: View<TextInput>) -> Self {
        Self { input }
    }
}

pub struct PrepaintState {
    lines: Option<Lines>,
    cursor: Option<PaintQuad>,
    selections: Option<Vec<PaintQuad>>,
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
        style.size.height = cx.line_height().into();
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

        let lines = Lines::new(lines_raw, cx.line_height());

        let selections: Option<Vec<PaintQuad>>;
        let paint_cursor: Option<PaintQuad>;

        if input.blink_manager.read(cx).show() {
            let line_idx = display_text.char_to_line(cursor);
            let char_idx = display_text.line_to_char(line_idx);

            paint_cursor = if let Some(cursor_pos) =
                lines.position_for_index_in_line(cursor - char_idx, line_idx)
            {
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos.x, bounds.top() + cursor_pos.y),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    rgb(0xcdd6f4),
                ))
            } else {
                None
            };
        } else {
            paint_cursor = None;
        }

        if selected_range.is_empty() {
            selections = None;
        } else {
            let start = display_text.char_to_byte(selected_range.start);
            let start_line_idx = display_text.char_to_line(start);
            let start_char_idx = display_text.line_to_char(start_line_idx);
            let start_point =
                lines.position_for_index_in_line(start - start_char_idx, start_line_idx);

            let end = display_text.char_to_byte(selected_range.end);
            let end_line_idx = display_text.char_to_line(end);
            let end_char_idx = display_text.line_to_char(end_line_idx);
            let end_point = lines.position_for_index_in_line(end - end_char_idx, end_line_idx);

            selections = match (start_point, end_point) {
                (Some(start), Some(end)) => {
                    let selection_color = rgba(0x7f849c64);
                    let line_count: u32 = ((end.y - start.y) / lines.line_height).round() as u32;

                    if line_count == 0 {
                        Some(vec![fill(
                            Bounds::from_corners(
                                point(bounds.left() + start.x, bounds.top() + start.y),
                                point(bounds.left() + end.x, bounds.bottom() + end.y),
                            ),
                            selection_color,
                        )])
                    } else if line_count == 1 {
                        let mut selections = Vec::new();
                        selections.push(fill(
                            Bounds::from_corners(
                                point(bounds.left() + start.x, bounds.top() + start.y),
                                point(bounds.right(), bounds.bottom() + start.y),
                            ),
                            selection_color,
                        ));
                        selections.push(fill(
                            Bounds::from_corners(
                                point(bounds.left(), bounds.top() + end.y),
                                point(bounds.left() + end.x, bounds.bottom() + end.y),
                            ),
                            selection_color,
                        ));
                        Some(selections)
                    } else {
                        let mut selections = Vec::new();
                        selections.push(fill(
                            Bounds::from_corners(
                                point(bounds.left() + start.x, bounds.top() + start.y),
                                point(bounds.right(), bounds.bottom() + start.y),
                            ),
                            selection_color,
                        ));

                        for n in 1..line_count {
                            selections.push(fill(
                                Bounds::from_corners(
                                    point(
                                        bounds.left(),
                                        bounds.top() + start.y + px(n as f32) * lines.line_height,
                                    ),
                                    point(
                                        bounds.right(),
                                        bounds.bottom()
                                            + start.y
                                            + px(n as f32) * lines.line_height,
                                    ),
                                ),
                                selection_color,
                            ));
                        }

                        selections.push(fill(
                            Bounds::from_corners(
                                point(bounds.left(), bounds.top() + end.y),
                                point(bounds.left() + end.x, bounds.bottom() + end.y),
                            ),
                            selection_color,
                        ));
                        Some(selections)
                    }
                }
                _ => None,
            }
        }

        PrepaintState {
            lines: Some(lines),
            cursor: paint_cursor,
            selections,
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
                cx.paint_quad(selection);
            }
        }

        let line_height = cx.line_height();
        let mut offset_y = px(0.);
        let lines = prepaint.lines.take().unwrap();
        for line in &lines.lines {
            line.paint(
                point(bounds.origin.x, bounds.origin.y + offset_y),
                line_height,
                cx,
            )
            .unwrap();
            offset_y += line_height;
        }

        if let Some(cursor) = prepaint.cursor.take() {
            cx.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(lines);
            input.last_bounds = Some(bounds);
        });
    }
}
