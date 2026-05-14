/// Screens available in the TUI navigation stack.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Screen {
    Connect,
    AddConnection,
    Database,
    Inspect,
    Records,
}

#[derive(Debug)]
pub struct Router {
    stack: Vec<Screen>,
}

impl From<Vec<Screen>> for Router {
    fn from(stack: Vec<Screen>) -> Self {
        Self { stack }
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            stack: vec![Screen::Connect],
        }
    }

    pub fn push(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    pub fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    pub fn current(&self) -> Option<&Screen> {
        self.stack.last()
    }
}
