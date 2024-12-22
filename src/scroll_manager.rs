use std::time::Duration;

use gpui::*;
use smallvec::{smallvec, SmallVec};

use crate::lines::Lines;

pub struct ScrollManager {
    show: bool,
    show_duration: Duration,
    show_epoch: usize,
    pub width: Pixels,
    pub offset: Point<Pixels>,
    padding_top: Pixels,
    padding_bottom: Pixels,
    padding_horizontal: Pixels,
}

impl ScrollManager {
    pub fn new() -> Self {
        Self {
            show: true,
            show_duration: Duration::from_millis(1000),
            show_epoch: 0,
            width: px(16.),
            offset: point(px(0.), px(0.)),
            padding_top: px(2.),
            padding_bottom: px(4.),
            padding_horizontal: px(32.),
        }
    }

    fn next_epoch(&mut self) -> usize {
        self.show_epoch += 1;
        self.show_epoch
    }

    fn show(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if epoch == self.show_epoch {
            self.show = true;
            cx.notify();

            let epoch = self.next_epoch();
            let duration = self.show_duration;
            cx.spawn(|this, mut cx| async move {
                Timer::after(duration).await;
                if let Some(this) = this.upgrade() {
                    this.update(&mut cx, |this, cx| this.hide(epoch, cx)).ok();
                }
            })
            .detach();
        }
    }

    fn hide(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if epoch == self.show_epoch {
            self.show = false;
            cx.notify();
        }
    }

    pub fn enable(&mut self, cx: &mut ModelContext<Self>) {
        self.show(self.show_epoch, cx);
    }

    pub fn is_in_scrollbar(&self, position: Point<Pixels>, bounds: &Bounds<Pixels>) -> bool {
        position.x >= bounds.right() - self.width
    }

    pub fn calc_offset_after_move(
        &mut self,
        line_idx: usize,
        cursor_x: Pixels,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
        cx: &mut ModelContext<Self>,
    ) {
        let cursor_screen_x = cursor_x + self.offset.x;

        self.offset.x = if cursor_screen_x < bounds.origin.x + self.padding_horizontal {
            px(0.).min(-(cursor_x - (bounds.origin.x + self.padding_horizontal)))
        } else if cursor_screen_x > bounds.origin.x + bounds.size.width - self.padding_horizontal {
            -(cursor_x - bounds.size.width + self.padding_horizontal)
        } else {
            self.offset.x
        };

        // adjust padding when bounds are smaller
        let (p_top, p_bottom) = if bounds.size.height
            < self.padding_top * lines.line_height + self.padding_bottom * lines.line_height
        {
            (px(0.), px(1.))
        } else {
            (self.padding_top, self.padding_bottom)
        };

        let cursor_y = lines.height_till_line_idx(line_idx) + lines.line_height;
        let margin_top = bounds.origin.y + lines.line_height * p_top;
        let margin_bottom = bounds.origin.y + bounds.size.height - lines.line_height * p_bottom;
        let lower_bound = self.offset.y.abs() + margin_top;
        let upper_bound = self.offset.y.abs() + margin_bottom;

        self.offset.y = if cursor_y < lower_bound {
            px(0.).min(-(cursor_y - margin_top))
        } else if cursor_y > upper_bound {
            -(cursor_y - margin_bottom)
        } else {
            self.offset.y
        };

        self.show(self.show_epoch, cx);
    }

    pub fn calc_offset_after_scroll(
        &mut self,
        delta: ScrollDelta,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
        cx: &mut ModelContext<Self>,
    ) {
        let d = delta.pixel_delta(lines.line_height);

        let content_width = lines.width();
        let upper_bound_x = -(content_width - bounds.size.width);
        self.offset.x = px(0.).min(upper_bound_x.max(self.offset.x + d.x));

        let upper_bound_y =
            -(lines.height() - bounds.size.height + lines.line_height * self.padding_bottom);
        self.offset.y = px(0.).min(upper_bound_y.max(self.offset.y + d.y));

        self.show(self.show_epoch, cx);

        cx.notify();
    }

    pub fn paint_bar(
        &self,
        bounds: Bounds<Pixels>,
        text_height: Pixels,
    ) -> Option<SmallVec<[PaintQuad; 2]>> {
        if bounds.size.height >= text_height || !self.show {
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
