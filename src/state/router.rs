#[derive(Debug)]
pub enum Screen {
    Connect,
    AddConnection,
    Schemas,
    Tables,
    TableView,
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
        self.stack.pop();
    }

    pub fn current(&self) -> Option<&Screen> {
        self.stack.last()
    }
}
