#[derive(Debug, Default)]
pub struct Resource {
    active: bool,
}

impl Resource {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn begin(&mut self) {
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
