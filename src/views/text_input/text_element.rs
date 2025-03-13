use gpui::*;
use ropey::{Rope, RopeSlice};
use smallvec::SmallVec;
use std::ops::Range;
use std::sync::Arc;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatch, TextProvider};

use crate::theme_manager::ActiveTheme;

use super::{
    lines::Lines,
    syntax::{LanguageConfig, LanguageConfigManager},
    text_input::TextInput,
};

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

    fn convert_byte_range_to_char_range(
        &self,
        range: &Range<usize>,
        display_text: &Rope,
    ) -> Range<usize> {
        Range {
            start: display_text.byte_to_char(range.start),
            end: display_text.byte_to_char(range.end),
        }
    }

    fn paint_range(
        &self,
        range: &Range<usize>,
        color: Rgba,
        display_text: &Rope,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
    ) -> Option<Vec<PaintQuad>> {
        if range.is_empty() {
            return None;
        }

        let start_byte_idx = display_text.char_to_byte(range.start);
        let start_line_idx = display_text.byte_to_line(start_byte_idx);
        let start_line_byte_idx = display_text.line_to_byte(start_line_idx);
        let start_byte_idx_in_line = start_byte_idx - start_line_byte_idx;
        let start_point =
            lines.position_for_byte_idx_in_line(start_byte_idx_in_line, start_line_idx);

        let end_byte_idx = display_text.char_to_byte(range.end);
        let end_line_idx = display_text.byte_to_line(end_byte_idx);
        let end_line_byte_idx = display_text.line_to_byte(end_line_idx);
        let end_point =
            lines.position_for_byte_idx_in_line(end_byte_idx - end_line_byte_idx, end_line_idx);

        let visual_line_count: u32 =
            ((end_point.y - start_point.y) / lines.line_height).round() as u32;
        let mut selection_quads = Vec::new();

        match visual_line_count {
            0 => {
                // Single-line selection
                selection_quads.push(fill(
                    self.get_line_selection_bounds(
                        bounds,
                        start_point,
                        end_point,
                        lines.line_height,
                    ),
                    color,
                ));
            }
            1 => {
                // Two-line selection
                let second_line_start = point(px(0.), end_point.y);
                let first_line_end = lines
                    .wrapped_line_end_point(start_line_idx, start_byte_idx_in_line)
                    .unwrap();

                selection_quads.push(fill(
                    self.get_line_selection_bounds(
                        bounds,
                        start_point,
                        first_line_end,
                        lines.line_height,
                    ),
                    color,
                ));
                selection_quads.push(fill(
                    self.get_line_selection_bounds(
                        bounds,
                        second_line_start,
                        end_point,
                        lines.line_height,
                    ),
                    color,
                ));
            }
            _ => {
                // Multi-line selection

                let mut first = true;

                for line_idx in start_line_idx..=end_line_idx {
                    let line = lines.line(line_idx).unwrap();
                    let height = lines.height_till_line_idx(line_idx);
                    let start_byte_idx_in_line = if first {
                        start_byte_idx - start_line_byte_idx
                    } else {
                        0
                    };
                    let end_byte_idx_in_line = end_byte_idx - display_text.line_to_byte(line_idx);

                    let wrapped_lines_end_idx = line
                        .wrap_boundaries()
                        .iter()
                        .map(|wb| {
                            let run = &line.unwrapped_layout.runs[wb.run_ix];
                            let glyph = &run.glyphs[wb.glyph_ix];
                            glyph.index
                        })
                        .filter(|byte_idx| {
                            start_byte_idx_in_line < *byte_idx && end_byte_idx_in_line > *byte_idx
                        });

                    let mut last_end_idx = 0;
                    for end_idx in wrapped_lines_end_idx {
                        let p = line.position_for_index(end_idx, lines.line_height).unwrap();
                        let pp = point(p.x, p.y + height);
                        selection_quads.push(fill(
                            self.get_line_selection_bounds(
                                bounds,
                                point(if first { start_point.x } else { px(0.) }, pp.y),
                                pp,
                                lines.line_height,
                            ),
                            color,
                        ));
                        last_end_idx = end_idx;
                        first = false;
                    }

                    if last_end_idx < end_byte_idx_in_line {
                        let end_idx = end_byte_idx_in_line.min(line.len());
                        let p = line.position_for_index(end_idx, lines.line_height).unwrap();
                        let pp = point(p.x, p.y + height);
                        selection_quads.push(fill(
                            self.get_line_selection_bounds(
                                bounds,
                                point(if first { start_point.x } else { px(0.) }, pp.y),
                                pp,
                                lines.line_height,
                            ),
                            color,
                        ));
                    }

                    first = false;
                }
            }
        }
        Some(selection_quads)
    }

    fn injection_pair<'a>(
        &self,
        query: &Query,
        query_match: &QueryMatch<'a, 'a>,
        source: &RopeSlice<'a>,
    ) -> (Option<&'a str>, Option<Node<'a>>) {
        let mut injection_capture = None;
        let mut content_node = None;

        for cap in query_match.captures {
            // todo: this should be part of some HighlighConfig thingy and done on init, since it does not change
            if let Some(capture_name) = query.capture_names().get(cap.index as usize) {
                match *capture_name {
                    "injection.language" => {
                        if let Some(name) = source.byte_slice(cap.node.byte_range()).as_str() {
                            injection_capture = Some(name);
                        }
                    }
                    "injection.content" => {
                        content_node = Some(cap.node);
                    }
                    _ => {}
                }
            }
        }

        (injection_capture, content_node)
    }

    fn query_tree(
        &self,
        query: &Query,
        tree: &tree_sitter::Tree,
        _base_run: &TextRun,
        rope_slice: RopeSlice,
        lang_config: &LanguageConfigManager,
    ) -> Vec<(Arc<LanguageConfig>, Range<usize>)> {
        let text = RopeProvider(rope_slice);

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, tree.root_node(), text);

        let mut injections = Vec::new();

        while let Some(mat) = matches.next() {
            let (mut injection_capture, content_node) =
                self.injection_pair(query, mat, &rope_slice);

            if injection_capture.is_none() {
                for prop in query.property_settings(mat.pattern_index) {
                    if prop.key.as_ref() == "injection.language" {
                        injection_capture = prop.value.as_ref().map(|s| s.as_ref());
                    }
                }
            }

            if let (Some(injection_capture), Some(content_node)) = (injection_capture, content_node)
            {
                if let Some(lang_config) =
                    lang_config.language_config_for_language_id(injection_capture)
                {
                    let r = content_node.range().start_byte..content_node.range().end_byte;
                    injections.push((lang_config, r));
                } else {
                    println!("missing language: {injection_capture}");
                }
            }
        }

        injections
    }

    fn highlight_node(
        &self,
        node: Node<'_>,
        name: Option<&str>,
        base_run: &TextRun,
    ) -> Option<TextRun> {
        let name = name?;
        let base_run = TextRun {
            len: node.end_byte() - node.start_byte(),
            ..base_run.clone()
        };

        match name {
            "text.strong" | "punctuation.special" => Some(TextRun {
                font: base_run.font.bold(),
                ..base_run
            }),
            "text.title" => Some(TextRun {
                font: base_run.font.bold(),
                color: hsla(0., 0., 0.8, 1.0),
                ..base_run
            }),
            "text.emphasis" => Some(TextRun {
                font: base_run.font.italic(),
                ..base_run
            }),
            "punctuation.delimiter" | "punctuation.bracket" => Some(TextRun {
                color: hsla(0., 0., 0.4, 1.0),
                ..base_run
            }),
            "property" | "tag" | "text.literal" | "text.uri" | "text.reference" => Some(TextRun {
                color: hsla(0.06, 0.92, 0.75, 1.0),
                ..base_run
            }),
            "number" => Some(TextRun {
                color: hsla(0.97, 0.65, 0.77, 1.0),
                ..base_run
            }),
            "boolean" => Some(TextRun {
                color: hsla(1., 0.92, 0.75, 1.0),
                ..base_run
            }),
            _ => {
                println!("missing: {name} in {node:?}");
                None
            }
        }
    }
}

pub struct PrepaintState {
    offset: Point<Pixels>,
    bounds: Bounds<Pixels>,
    lines: Option<Lines>,
    cursor: Option<PaintQuad>,
    selections: Option<Vec<PaintQuad>>,
    highlights: Option<Vec<PaintQuad>>,
    scroll_bar: Option<SmallVec<[PaintQuad; 2]>>,
    scroll_bar_hitbox: Hitbox,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}
impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

#[derive(Clone)]
pub struct RopeProvider<'a>(pub RopeSlice<'a>);
impl<'a> TextProvider<&'a [u8]> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
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
        (): &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = cx.text_style();
        let text_color = style.color;
        let display_text = input.content.clone();

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

        let runs = if input.parse_tree.is_some() {
            let mut runs = vec![];
            let tree = input.parse_tree.clone().unwrap();

            let markdown = input
                .language_configs
                .language_config_for_language_id("markdown")
                .expect("Markdown should always be there");

            let md_query = markdown.injection_query.as_ref().unwrap();
            let configs = self.query_tree(
                md_query,
                &tree,
                &run,
                display_text.slice(..),
                &input.language_configs,
            );

            let mut cursor = QueryCursor::new();
            let mut captures = cursor.captures(
                &markdown.highlight_query,
                tree.root_node(),
                RopeProvider(display_text.slice(..)),
            );

            let mut last_end: usize = 0;

            while let Some((mat, _)) = captures.next() {
                for cap in mat.captures {
                    for (config, range) in &configs {
                        let node_start = cap.node.start_byte();
                        if node_start < range.start || last_end > range.start {
                            continue;
                        }

                        let mut parser = Parser::new();
                        parser
                            .set_language(&config.language)
                            .expect("Error Loading Grammar");

                        let text = display_text.slice(range.clone());
                        let Some(parse_tree) = parser.parse_with(
                            &mut |byte, _| {
                                let (chunk, start_byte, _, _) = text.chunk_at_byte(byte);
                                &chunk.as_bytes()[byte - start_byte..]
                            },
                            None,
                        ) else {
                            continue;
                        };

                        let mut cursor = QueryCursor::new();
                        let mut captures = cursor.captures(
                            &config.highlight_query,
                            parse_tree.root_node(),
                            RopeProvider(text),
                        );

                        while let Some((mat, _)) = captures.next() {
                            for cap in mat.captures {
                                if range.start + cap.node.start_byte() < last_end {
                                    continue;
                                }

                                if let Some(r) = self.highlight_node(
                                    cap.node,
                                    config
                                        .highlight_query
                                        .capture_names()
                                        .get(cap.index as usize)
                                        .copied(),
                                    &run,
                                ) {
                                    if range.start + cap.node.start_byte() > last_end {
                                        runs.push(TextRun {
                                            len: range.start + cap.node.start_byte() - last_end,
                                            ..run.clone()
                                        });
                                    }

                                    runs.push(r);
                                    last_end = range.start + cap.node.end_byte();
                                }
                            }
                        }
                        if last_end < range.end {
                            runs.push(TextRun {
                                len: range.end - last_end,
                                ..run.clone()
                            });
                            last_end = range.end;
                        }
                    }

                    if last_end > cap.node.start_byte() {
                        continue;
                    }

                    if let Some(r) = self.highlight_node(
                        cap.node,
                        markdown
                            .highlight_query
                            .capture_names()
                            .get(cap.index as usize)
                            .copied(),
                        &run,
                    ) {
                        if cap.node.start_byte() > last_end {
                            runs.push(TextRun {
                                len: cap.node.start_byte() - last_end,
                                ..run.clone()
                            });
                        }

                        runs.push(r);
                        last_end = cap.node.end_byte();
                    }
                }
            }

            if last_end < display_text.len_bytes() {
                runs.push(TextRun {
                    len: display_text.len_bytes() - last_end,
                    ..run.clone()
                });
            }

            runs
        } else {
            vec![run]
        };

        // let runs = if let Some(marked_range) = input.marked_range.as_ref() {
        //     vec![
        //         TextRun {
        //             len: marked_range.start,
        //             ..run.clone()
        //         },
        //         TextRun {
        //             len: marked_range.end - marked_range.start,
        //             underline: Some(UnderlineStyle {
        //                 color: Some(run.color),
        //                 thickness: px(1.),
        //                 wavy: false,
        //             }),
        //             ..run.clone()
        //         },
        //         TextRun {
        //             len: input.content.len_bytes() - marked_range.end,
        //             ..run.clone()
        //         },
        //     ]
        //     .into_iter()
        //     .filter(|run| run.len > 0)
        //     .collect()
        // } else {
        //     vec![run]
        // };

        let font_size = style.font_size.to_pixels(cx.rem_size());
        let text: SharedString = display_text.to_string().into();

        let lines_raw = cx
            .text_system()
            .shape_text(
                text.clone(),
                font_size,
                &runs,
                if input.soft_wrap_enabled() {
                    Some(new_bounds.size.width)
                } else {
                    None
                },
            )
            .unwrap();

        let line_height = cx.line_height();
        let lines = Lines::new(lines_raw, line_height);

        let line_idx = display_text.char_to_line(cursor);
        let line_byte_idx = display_text.line_to_byte(line_idx);
        let cursor_byte_idx = display_text.char_to_byte(cursor);
        let cursor_pos =
            lines.position_for_byte_idx_in_line(cursor_byte_idx - line_byte_idx, line_idx);

        let scroll_manager = input.scroll_manager.read(cx);
        let offset = scroll_manager.offset(input.soft_wrap_enabled());
        let scroll_bar = input.scroll_manager.read_with(cx, |this, cx| {
            this.paint_bar(&bounds, lines.height(), cursor_pos, cx)
        });

        let paint_cursor = if input.blink_manager.read(cx).show() {
            Some(fill(
                Bounds::new(
                    point(
                        new_bounds.left() + cursor_pos.x,
                        new_bounds.top() + cursor_pos.y,
                    ),
                    size(px(2.), line_height),
                ),
                cx.theme().cursor,
            ))
        } else {
            None
        };

        let selection_color = cx.theme().selection_bg;
        let selections = self.paint_range(
            &selected_range,
            selection_color,
            &display_text,
            &lines,
            &new_bounds,
        );

        let highlights: Vec<PaintQuad> = input
            .highlights
            .iter()
            .filter_map(|range| {
                self.paint_range(
                    &self.convert_byte_range_to_char_range(range, &display_text),
                    rgb(0x000000),
                    &display_text,
                    &lines,
                    &new_bounds,
                )
            })
            .flatten()
            .collect();

        PrepaintState {
            offset,
            bounds: new_bounds,
            lines: Some(lines),
            cursor: paint_cursor,
            selections,
            highlights: Some(highlights),
            scroll_bar,
            scroll_bar_hitbox: cx.insert_hitbox(scroll_manager.bounds(&bounds), false),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        cx.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
        );

        let lines = prepaint.lines.take().unwrap();

        cx.with_content_mask(Some(ContentMask { bounds }), |cx| {
            if let Some(highlights) = prepaint.highlights.take() {
                for selection in highlights {
                    let mut selection = selection.clone();
                    selection.bounds.origin.x = prepaint.offset.x + selection.bounds.origin.x;
                    selection.bounds.origin.y = prepaint.offset.y + selection.bounds.origin.y;
                    cx.paint_quad(selection);
                }
            }

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
        });

        cx.set_cursor_style(CursorStyle::Arrow, &prepaint.scroll_bar_hitbox);

        self.input.update(cx, |input, cx| {
            input.last_layout = Some(lines);
            input.last_bounds = Some(prepaint.bounds);
            input.last_offset = Some(prepaint.offset);

            input.notify_about_paint(cx);
        });
    }
}
