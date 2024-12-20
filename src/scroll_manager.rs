use gpui::*;
use smallvec::{smallvec, SmallVec};

pub struct ScrollManager {
    pub width: Pixels,
}

impl ScrollManager {
    pub fn new() -> Self {
        Self { width: px(16.) }
    }

    pub fn paint_bar(
        &self,
        bounds: Bounds<Pixels>,
        text_height: Pixels,
        offset: Point<Pixels>,
    ) -> Option<SmallVec<[PaintQuad; 2]>> {
        if bounds.size.height >= text_height {
            return None;
        }

        let min = px(42.);
        let max = bounds.size.height * 0.9;
        let raw_height = (bounds.size.height / text_height) * bounds.size.height;
        let height = raw_height / bounds.size.height * (max - min) + min;

        let offset_y = (bounds.size.height - height - px(2.0))
            * ((offset.y.abs()) / (text_height - bounds.size.height * 0.8));

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
