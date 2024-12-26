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
        self.lines
            .iter()
            .enumerate()
            .take_while(|(idx, _)| *idx < line_idx)
            .fold(px(0.), |total_height, (_, line)| {
                total_height + line.size(self.line_height).height
            })
    }

    pub fn index_for_position(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let mut previous_heights = px(0.);
        for (idx, line) in self.lines.iter().enumerate() {
            let size = line.size(self.line_height);
            let temp_pos = point(position.x, position.y - previous_heights);

            if temp_pos.y < px(0.) {
                return None;
            }

            if let Ok(index) = line.index_for_position(temp_pos, self.line_height) {
                return Some((idx, index));
            }

            previous_heights += size.height;
        }

        None
    }

    pub fn line_idx_for_y(&self, y: Pixels) -> Option<usize> {
        let mut previous_heights = px(0.);
        for (idx, line) in self.lines.iter().enumerate() {
            let size = line.size(self.line_height);

            if y >= previous_heights && y <= previous_heights + size.height {
                return Some(idx);
            }

            previous_heights += size.height;
        }

        None
    }

    pub fn height(&self) -> Pixels {
        self.lines.iter().fold(px(0.), |height, line| {
            height + line.size(self.line_height).height
        })
    }

    pub fn width(&self) -> Pixels {
        self.lines.iter().fold(px(0.), |max_width, line| {
            max_width.max(line.size(self.line_height).width)
        })
    }

    pub fn line(&self, line_number: usize) -> Option<&WrappedLine> {
        self.lines.get(line_number)
    }
}
