/* use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
}; */

#[derive(Debug)]
enum Screen {
    Connect,
    Schemas,
    Tables,
    TableView,
}

#[derive(Debug)]
struct Router {
    stack: Vec<Screen>,
}

impl From<Vec<Screen>> for Router {
    fn from(stack: Vec<Screen>) -> Self {
        Self { stack }
    }
}

impl Router {
    pub fn push(&mut self, screen: Screen) {
        self.stack.push(screen);
    }
    pub fn pop(&mut self) {
        self.stack.pop();
    }
    pub fn current(&self) -> Option<&Screen> {
        self.stack.last()
    }
    pub fn current_mut(&mut self) -> Option<&mut Screen> {
        self.stack.last_mut()
    }
}
