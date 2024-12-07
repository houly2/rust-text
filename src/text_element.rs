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

    fn find_position(
        &self,
        lines: &SmallVec<[WrappedLine; 1]>,
        position: usize,
        line_height: Pixels,
    ) -> Option<Point<Pixels>> {
        for line in lines {
            let a = line.position_for_index(position, line_height);
            if let Some(b) = a {
                return Some(b);
            }
        }

        None
    }
}

pub struct PrepaintState {
    lines: Option<SmallVec<[WrappedLine; 1]>>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
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

        let line_height = cx.line_height();
        let lines = cx
            .text_system()
            .shape_text(text.clone(), font_size, &runs, None)
            .unwrap();

        let selection: Option<PaintQuad>;
        let paint_cursor: Option<PaintQuad>;

        if input.blink_manager.read(cx).show() {
            paint_cursor = if let Some(cursor_pos) = self.find_position(&lines, cursor, line_height)
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
            selection = None;
        } else {
            let start = display_text.char_to_byte(selected_range.start);
            let end = display_text.char_to_byte(selected_range.end);

            let start_point = self.find_position(&lines, start, line_height);
            let end_point = self.find_position(&lines, end, line_height);

            selection = match (start_point, end_point) {
                (Some(start), Some(end)) => Some(fill(
                    Bounds::from_corners(
                        point(bounds.left() + start.x, bounds.top() + start.y),
                        point(bounds.left() + end.x, bounds.bottom() + end.y),
                    ),
                    rgba(0x7f849c64),
                )),
                _ => None,
            }
        }

        PrepaintState {
            lines: Some(lines),
            cursor: paint_cursor,
            selection,
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
        if let Some(selection) = prepaint.selection.take() {
            cx.paint_quad(selection);
        }

        let line_height = cx.line_height();
        let mut offset_y = px(0.);
        let lines = prepaint.lines.take().unwrap();
        for line in lines {
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
            // input.last_layout = Some(lines);
            input.last_bounds = Some(bounds);
        });
    }
}
