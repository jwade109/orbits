use crate::commands::command::CommandDecl;
use crate::input::InputState;
use bevy::input::keyboard::Key;
use bevy::input::ButtonState;

pub struct DebugConsole {
    is_active: bool,
    text: String,
    history: Vec<String>,
}

impl DebugConsole {
    pub fn new() -> Self {
        Self {
            is_active: false,
            text: String::new(),
            history: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.is_active = true;
    }

    pub fn hide(&mut self) {
        self.is_active = false;
    }

    pub fn toggle(&mut self) {
        self.is_active = !self.is_active;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn cmd(&self) -> &str {
        &self.text
    }

    pub fn lines(&self) -> &Vec<String> {
        &self.history
    }

    pub fn log(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.history.push(s);
    }

    fn enter(&mut self) -> Option<(CommandDecl, Vec<String>)> {
        if self.text.is_empty() {
            return None;
        }
        let cmd = self.text.clone();
        self.history.push("".into());
        self.history.push(format!("> {}", cmd));
        self.text.clear();

        match shellwords::split(&cmd) {
            Ok(args) => {
                let name = args.get(0).cloned().unwrap_or("".to_string());
                let v = CommandDecl::from_str(&name);
                if let Some(v) = v {
                    Some((v, args))
                } else {
                    self.print(format!("No command named \"{}\"", name));
                    None
                }
            }
            Err(e) => {
                self.print(format!("{:?}", e));
                None
            }
        }
    }

    pub fn print(&mut self, lines: impl Into<String>) {
        let lines = lines.into();
        for line in lines.lines() {
            for wrapped in textwrap::wrap(line, 80) {
                self.history.push(wrapped.to_string());
            }
        }
    }

    fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn process_input(&mut self, input: &InputState) -> Option<(CommandDecl, Vec<String>)> {
        if !self.is_active {
            return None;
        }

        for key in &input.keyboard_events {
            match key.state {
                ButtonState::Pressed => (),
                ButtonState::Released => continue,
            };

            match &key.logical_key {
                Key::Character(c) => {
                    // TODO handle this better
                    if c == "`" {
                        continue;
                    }
                    self.text += c;
                }
                Key::Enter => return self.enter(),
                Key::Backspace => self.backspace(),
                Key::Space => self.text += " ",
                _ => (),
            }
        }
        None
    }
}
