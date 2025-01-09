use std::time::Duration;

use gpui::*;
use smallvec::{smallvec, SmallVec};

use crate::{theme_manager::ActiveTheme, views::text_input::lines::Lines};

pub struct ScrollManager {
    show: bool,
    show_duration: Duration,
    show_epoch: usize,
    calc_epoch: usize,
    pub width: Pixels,
    pub offset: Point<Pixels>,
    padding_vertical: Pixels,
    padding_horizontal: Pixels,
}

impl ScrollManager {
    pub fn new() -> Self {
        Self {
            show: true,
            show_duration: Duration::from_millis(1000),
            show_epoch: 0,
            calc_epoch: 0,
            width: px(16.),
            offset: point(px(0.), px(0.)),
            padding_vertical: px(2.),
            padding_horizontal: px(32.),
        }
    }

    fn next_show_epoch(&mut self) -> usize {
        self.show_epoch += 1;
        self.show_epoch
    }

    pub fn next_calc_epoch(&mut self) -> usize {
        self.calc_epoch += 1;
        self.calc_epoch
    }

    fn show(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if epoch == self.show_epoch {
            self.show = true;
            cx.notify();

            let epoch = self.next_show_epoch();
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
        epoch: usize,
        line_idx: usize,
        cursor_byte_idx: usize,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
        cx: &mut ModelContext<Self>,
    ) {
        if epoch != self.calc_epoch {
            return;
        }

        let cursor_pos = lines.position_for_byte_idx_in_line(cursor_byte_idx, line_idx);

        let cursor_screen_x = cursor_pos.x + self.offset.x;

        self.offset.x = if cursor_screen_x < bounds.origin.x + self.padding_horizontal {
            px(0.).min(-(cursor_pos.x - (bounds.origin.x + self.padding_horizontal)))
        } else if cursor_screen_x > bounds.origin.x + bounds.size.width - self.padding_horizontal {
            -(cursor_pos.x - bounds.size.width + self.padding_horizontal)
        } else {
            self.offset.x
        };

        // adjust padding when bounds are smaller
        let padding_vertical = if bounds.size.height
            < self.padding_vertical * lines.line_height + self.padding_vertical * lines.line_height
        {
            px(1.)
        } else {
            self.padding_vertical
        };

        // let cursor_y = lines.height_till_line_idx(line_idx) + cursor_pos.y;
        let cursor_y = bounds.origin.y + cursor_pos.y + px(lines.line_height / px(2.));

        let margin_top = bounds.origin.y + lines.line_height * padding_vertical;
        let margin_bottom =
            bounds.origin.y + bounds.size.height - lines.line_height * padding_vertical;
        let lower_bound = self.offset.y.abs() + margin_top;
        let upper_bound = self.offset.y.abs() + margin_bottom;

        let old_offset_y = self.offset.y;

        self.offset.y = if cursor_y < lower_bound {
            px(0.).min(-(cursor_y - margin_top))
        } else if cursor_y > upper_bound {
            -(cursor_y - margin_bottom)
        } else {
            self.offset.y
        };

        if old_offset_y != self.offset.y {
            self.show(self.show_epoch, cx);
            cx.notify();
        }
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
            -(lines.height() - bounds.size.height + lines.line_height * self.padding_vertical);
        self.offset.y = px(0.).min(upper_bound_y.max(self.offset.y + d.y));

        self.show(self.show_epoch, cx);

        cx.notify();
    }

    pub fn scroll_to(
        &mut self,
        position: Point<Pixels>,
        lines: &Lines,
        bounds: &Bounds<Pixels>,
        cx: &mut ModelContext<Self>,
    ) {
        let height = lines.height();
        let percentage = (position.y - bounds.origin.y) / bounds.size.height;
        self.offset.y = px(0.).min(-(height - bounds.size.height / 2.) * percentage);

        self.show(self.show_epoch, cx);
        cx.notify();
    }

    pub fn bounds(&self, bounds: &Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::new(
            point(bounds.right() - self.width, bounds.top()),
            size(self.width, bounds.size.height),
        )
    }

    pub fn paint_bar(
        &self,
        bounds: &Bounds<Pixels>,
        text_height: Pixels,
        cursor_pos: Point<Pixels>,
        cx: &AppContext,
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

        let own_bounds = self.bounds(bounds);
        let cursor_bar_pos =
            own_bounds.top() + (own_bounds.size.height * (cursor_pos.y / text_height));

        let theme = cx.theme();

        Some(smallvec![
            quad(
                own_bounds,
                Corners {
                    top_left: px(0.),
                    top_right: px(0.),
                    bottom_right: px(0.),
                    bottom_left: px(0.),
                },
                theme.scroll_bar_bg,
                Edges {
                    top: px(0.),
                    right: px(0.),
                    bottom: px(0.),
                    left: px(1.),
                },
                theme.scroll_bar_border,
            ),
            quad(
                Bounds::new(
                    point(
                        own_bounds.left() + px(4.),
                        own_bounds.top() + px(1.) + offset_y
                    ),
                    size(self.width - px(8.), height),
                ),
                Corners {
                    top_left: px(3.),
                    top_right: px(3.),
                    bottom_right: px(3.),
                    bottom_left: px(3.),
                },
                theme.scroll_bar_handle_bg,
                Edges {
                    top: px(0.),
                    right: px(0.),
                    bottom: px(0.),
                    left: px(0.),
                },
                transparent_black()
            ),
            fill(
                Bounds::new(
                    point(own_bounds.left() + px(3.), cursor_bar_pos),
                    size(self.width - px(6.), px(2.))
                ),
                theme.scroll_bar_cursor_highlight
            )
        ])
    }
}
