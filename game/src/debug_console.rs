use crate::commands::{command::Command, example::ExampleCommand};
use crate::input::*;
use bevy::input::keyboard::Key;
use bevy::input::ButtonState;
use clap::Parser;

#[derive(Debug, Clone)]
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

    fn enter(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let cmd = self.text.clone();
        self.history.push("".into());
        self.history.push(format!("> {}", cmd));
        self.text.clear();
        self.on_command(cmd);
    }

    fn print(&mut self, lines: impl Into<String>) {
        let lines = lines.into();
        for line in lines.lines() {
            for wrapped in textwrap::wrap(line, 80) {
                self.history.push(wrapped.to_string());
            }
        }
    }

    fn on_command(&mut self, cmd: String) {
        if cmd == "reflect" {
            let s = format!("{:#?}", self);
            self.print(s);
        } else if cmd == "clear" {
            self.history.clear();
        } else if cmd == "hello" {
            self.print("Hi there!\nAnd another!");
        } else if cmd == "pwd" {
            self.print(format!("{:?}", std::env::current_dir()));
        } else if cmd == "args" {
            self.print(format!("{:?}", std::env::args()));
        } else if cmd == "env" {
            for v in std::env::vars() {
                self.print(format!("{}: {}", v.0, v.1));
            }
        } else {
            let args = shellwords::split(&cmd);
            if let Ok(args) = args {
                match ExampleCommand::try_parse_from(args) {
                    Ok(e) => self.print(format!("{:?}", e)),
                    Err(e) => self.print(format!("{}", e)),
                }
            } else {
                self.print(format!("{:?}", args));
            }
        }
    }

    fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn process_input(&mut self, input: &InputState) {
        if !self.is_active {
            return;
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
                Key::Enter => self.enter(),
                Key::Backspace => self.backspace(),
                Key::Space => self.text += " ",
                _ => (),
            }
        }
    }
}
