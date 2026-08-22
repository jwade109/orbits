pub struct EventBus {
    font_id: Option<usize>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { font_id: None }
    }

    pub fn clicked(&mut self, font_id: usize) {
        self.font_id = Some(font_id);
    }

    pub fn new_font_id(&self) -> Option<usize> {
        self.font_id
    }
}
