use crate::util::Image;

pub enum UndoFrame {
    Rotate(i32),
    FlipHorizontal,
    FlipVertical,
    Crop { frames: Vec<Image>, rotation: i32 },
    Resize(Vec<Image>),
    Color(Vec<Image>),
    ColorSpace(Vec<Image>),
}

impl UndoFrame {
    fn is_edit(&self) -> bool {
        match self {
            UndoFrame::Rotate(..) => false,
            UndoFrame::FlipHorizontal => false,
            UndoFrame::FlipVertical => false,
            UndoFrame::Crop { .. } => true,
            UndoFrame::Resize(..) => true,
            UndoFrame::Color(..) => true,
            UndoFrame::ColorSpace(..) => true,
        }
    }
}

pub struct UndoStack {
    stack: Vec<UndoFrame>,
    index: usize,
    saved: bool,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            index: 0,
            saved: true,
        }
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.index = 0;
        self.saved = true;
    }

    pub fn push(&mut self, frame: UndoFrame) {
        self.stack.truncate(self.stack.len() - self.index);
        self.index = 0;
        if frame.is_edit() {
            self.saved = false;
        }
        self.stack.push(frame);
    }

    pub fn undo(&mut self) -> Option<&mut UndoFrame> {
        if self.stack.len() - self.index > 0 {
            self.index += 1;
            let index = self.stack.len() - self.index;
            let frame = &mut self.stack[index];
            if frame.is_edit() {
                self.saved = false;
            }
            Some(frame)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&mut UndoFrame> {
        if self.index > 0 {
            let index = self.stack.len() - self.index;
            self.index -= 1;
            let frame = &mut self.stack[index];
            if frame.is_edit() {
                self.saved = false;
            }
            Some(frame)
        } else {
            None
        }
    }

    pub fn is_edited(&self) -> bool {
        if self.saved {
            return false;
        }
        self.stack.iter().any(|s| s.is_edit())
    }

    pub fn set_saved(&mut self) {
        self.saved = true;
    }
}
