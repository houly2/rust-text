use gpui::*;
use smallvec::SmallVec;

pub struct Lines {
    pub lines: SmallVec<[WrappedLine; 1]>,
    pub line_height: Pixels,
}

impl Lines {
    pub fn new(lines: SmallVec<[WrappedLine; 1]>, line_height: Pixels) -> Self {
        Self { lines, line_height }
    }

    pub fn position_for_index_in_line(&self, index: usize, line_number: usize) -> Point<Pixels> {
        let previous_heights = self.height_till_line_idx(line_number);
        let line = self.lines.get(line_number).unwrap();
        let position_in_line = line.position_for_index(index, self.line_height).unwrap();
        point(position_in_line.x, position_in_line.y + previous_heights)
    }

    pub fn height_till_line_idx(&self, line_idx: usize) -> Pixels {
        let mut heights = px(0.);
        for n in 0..line_idx {
            if let Some(line) = self.lines.get(n) {
                heights += line.size(self.line_height).height;
            }
        }
        heights
    }

    pub fn index_for_position(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let mut previous_heights = px(0.);
        let mut line_idx = 0;
        for line in &self.lines {
            let size = line.size(self.line_height);
            let temp_pos = point(position.x, position.y - previous_heights);

            if temp_pos.y < px(0.) {
                return None;
            }

            match line.index_for_position(temp_pos, self.line_height) {
                Ok(v) => return Some((line_idx, v)),
                _ => {}
            }

            line_idx += 1;
            previous_heights += size.height;
        }

        None
    }

    pub fn line_idx_for_y(&self, y: Pixels) -> Option<usize> {
        let mut previous_heights = px(0.);
        let mut line_idx = 0;
        for line in &self.lines {
            let size = line.size(self.line_height);

            if y >= previous_heights && y <= previous_heights + size.height {
                return Some(line_idx);
            }

            line_idx += 1;
            previous_heights += size.height;
        }

        None
    }

    pub fn height(&self) -> Pixels {
        let mut height = px(0.);
        for line in &self.lines {
            let size = line.size(self.line_height);
            height += size.height;
        }
        height
    }

    pub fn width(&self) -> Pixels {
        let mut max_width = px(0.);
        for line in &self.lines {
            let size = line.size(self.line_height);
            max_width = max_width.max(size.width);
        }
        max_width
    }
}
