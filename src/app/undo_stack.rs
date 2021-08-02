use util::Image;

pub enum UndoFrame {
    Rotate(i32),
    FlipHorizontal,
    FlipVertical,
    Crop {
        frames: Vec<Image>,
        rotation: i64,
    },
}

pub struct UndoStack {
    stack: Vec<UndoFrame>,
    index: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            index: 0,
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.index = 0;
    }

    pub fn push(&mut self, item: UndoFrame) {
        self.stack.truncate(self.stack.len() - self.index);
        self.index = 0;
        self.stack.push(item);
    }

    pub fn undo(&mut self) -> Option<&mut UndoFrame> {
        if self.stack.len() - self.index > 0 {
            self.index += 1;
            let index = self.stack.len() - self.index;
            Some(&mut self.stack[index])
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&mut UndoFrame> {
        if self.index > 0 {
            let index = self.stack.len() - self.index;
            self.index -= 1;
            Some(&mut self.stack[index])
        }else {
            None
        }
    }
}