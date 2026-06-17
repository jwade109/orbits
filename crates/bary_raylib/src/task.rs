pub enum TaskStatus {
    Continue(String),
    Done,
}

pub struct ExampleTask {
    progress: usize,
    necessary_steps: usize,
}

impl ExampleTask {
    pub fn work(&mut self) -> TaskStatus {
        self.progress += 1;

        if self.progress == self.necessary_steps {
            TaskStatus::Done
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
            TaskStatus::Continue("Working...".to_string())
        }
    }
}
