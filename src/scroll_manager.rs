use gpui::*;
use smallvec::{smallvec, SmallVec};
use std::cmp;

use crate::lines::Lines;

pub struct ScrollManager {
    pub width: Pixels,
    pub offset: Point<Pixels>,
}

impl ScrollManager {
    pub fn new() -> Self {
        Self {
            width: px(16.),
            offset: point(px(0.), px(0.)),
        }
    }

    pub fn calc_offset_after_move(
        &mut self,
        line_idx: usize,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
    ) {
        let position_in_height = lines.height_till_line_idx(line_idx) + lines.line_height;
        let substract_height_lower = bounds.origin.y + bounds.size.height * 0.1; // todo: should be in line_height
        let substract_height_upper = bounds.origin.y + bounds.size.height * 0.8; // todo: should be in line_height
        let lower_bound = self.offset.y.abs() + substract_height_lower;
        let upper_bound = self.offset.y.abs() + substract_height_upper;

        if position_in_height < lower_bound {
            self.offset.y = cmp::min(-(position_in_height - substract_height_lower), px(0.));
        }

        if position_in_height > upper_bound {
            self.offset.y = -(position_in_height - substract_height_upper);
        }

        // todo: x offset
    }

    pub fn calc_offset_after_scroll(
        &mut self,
        delta: ScrollDelta,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
        cx: &mut ModelContext<Self>,
    ) {
        let d = delta.pixel_delta(lines.line_height);
        let upper_bound = -(lines.height() - bounds.size.height * 0.9);
        self.offset.y = cmp::min(px(0.), cmp::max(upper_bound, self.offset.y + d.y));

        // todo: x offset
        cx.notify();
    }

    pub fn paint_bar(
        &self,
        bounds: Bounds<Pixels>,
        text_height: Pixels,
    ) -> Option<SmallVec<[PaintQuad; 2]>> {
        if bounds.size.height >= text_height {
            return None;
        }

        let min = px(42.);
        let max = bounds.size.height * 0.9;
        let raw_height = (bounds.size.height / text_height) * bounds.size.height;
        let height = raw_height / bounds.size.height * (max - min) + min;

        let offset_y = (bounds.size.height - height - px(2.0))
            * ((self.offset.y.abs()) / (text_height - bounds.size.height * 0.8));

        Some(smallvec![
            quad(
                Bounds::new(
                    point(bounds.right() - self.width, bounds.top()),
                    size(self.width, bounds.size.height),
                ),
                Corners {
                    top_left: px(0.),
                    top_right: px(0.),
                    bottom_right: px(0.),
                    bottom_left: px(0.),
                },
                rgba(0x14142033),
                Edges {
                    top: px(0.),
                    right: px(0.),
                    bottom: px(0.),
                    left: px(1.),
                },
                rgba(0x2e2e4d66),
            ),
            quad(
                Bounds::new(
                    point(
                        bounds.right() - self.width + px(3.),
                        bounds.top() + px(1.) + offset_y
                    ),
                    size(self.width - px(6.), height),
                ),
                Corners {
                    top_left: px(2.),
                    top_right: px(2.),
                    bottom_right: px(2.),
                    bottom_left: px(2.),
                },
                rgba(0xaeaecd66),
                Edges {
                    top: px(1.),
                    right: px(1.),
                    bottom: px(1.),
                    left: px(1.),
                },
                rgba(0x2e2e4d00),
            )
        ])
    }
}
