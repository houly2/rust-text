use std::ops::Range;

use ropey::Rope;

pub trait Command {
    fn execute(&self, content: &mut Rope) -> Range<usize>;
    fn undo(&self, content: &mut Rope) -> Range<usize>;
    fn char_range(&self) -> Range<usize>;
}

pub struct InsertCommand {
    position: usize,
    text: String,
    old_selection: Range<usize>,
}

impl InsertCommand {
    pub fn new(position: usize, text: String, old_selection: Range<usize>) -> Self {
        Self {
            position,
            text,
            old_selection,
        }
    }
}

impl Command for InsertCommand {
    fn execute(&self, content: &mut Rope) -> Range<usize> {
        content.insert(self.position, &self.text);
        let new_pos = self.position + self.text.chars().count();
        new_pos..new_pos
    }

    fn undo(&self, content: &mut Rope) -> Range<usize> {
        content.remove(self.position..self.position + self.text.chars().count());
        self.old_selection.clone()
    }

    fn char_range(&self) -> Range<usize> {
        self.position..self.position + self.text.chars().count()
    }
}

pub struct DeleteCommand {
    position: usize,
    text: String,
    old_selection: Range<usize>,
}

impl DeleteCommand {
    pub fn new(position: usize, text: String, old_selection: Range<usize>) -> Self {
        Self {
            position,
            text,
            old_selection,
        }
    }
}

impl Command for DeleteCommand {
    fn execute(&self, content: &mut Rope) -> Range<usize> {
        content.remove(self.position..self.position + self.text.chars().count());
        self.position..self.position
    }

    fn undo(&self, content: &mut Rope) -> Range<usize> {
        content.insert(self.position, &self.text);
        self.old_selection.clone()
    }

    fn char_range(&self) -> Range<usize> {
        self.position..self.position - self.text.chars().count()
    }
}
