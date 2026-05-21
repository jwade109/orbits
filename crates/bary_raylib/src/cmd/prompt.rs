use super::commands::*;
use crate::Action;

pub enum Severity {
    Info,
    Error,
}

pub struct CommandPrompt {
    pub contents: String,
    pub is_active: bool,
    pub lines: Vec<(String, Severity)>,
    pub suggest_text: String,
    pub commands: Vec<Command>,
}

impl CommandPrompt {
    pub fn new() -> Self {
        Self {
            contents: String::new(),
            is_active: false,
            lines: Vec::new(),
            suggest_text: String::new(),
            commands: all_commands(),
        }
    }

    pub fn on_event(&mut self, event: &rdev::Event) -> Option<Action> {
        if let rdev::EventType::KeyPress(k) = &event.event_type {
            match k {
                rdev::Key::Backspace => self.on_backspace(),
                rdev::Key::Return => return self.on_enter(),
                rdev::Key::BackQuote => self.focus(),
                rdev::Key::Escape => self.dismiss(),
                rdev::Key::Tab => self.on_tab_complete(),
                _ => {
                    if let Some(n) = &event.name {
                        if n.is_ascii() {
                            self.append(n);
                        }
                    }
                }
            }
        }

        None
    }

    pub fn on_backspace(&mut self) {
        if !self.is_active {
            return;
        }
        self.contents.pop();
        self.update_suggest_text();
    }

    fn error(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.lines.push((s, Severity::Error));
    }

    fn on_enter(&mut self) -> Option<Action> {
        if !self.is_active {
            return None;
        }
        if self.contents.is_empty() {
            return None;
        }

        self.lines.push((self.contents.clone(), Severity::Info));

        let mut ret = None;

        if let Some(cmd) = self.find_best_command() {
            match cmd.parse(&self.get_args()) {
                Ok(action) => {
                    ret = Some(action);
                }
                Err(e) => {
                    self.error(format!("Failed to parse: {e:?}"));
                }
            }
        } else {
            self.error(format!("Bad command"));
        }

        self.contents.clear();
        self.update_suggest_text();

        ret
    }

    pub fn append(&mut self, s: &str) {
        if !self.is_active {
            return;
        }
        self.contents += s;
        self.update_suggest_text();
    }

    pub fn on_tab_complete(&mut self) {
        self.contents = self.suggest_text.clone();
    }

    pub fn focus(&mut self) {
        self.is_active = true;
        self.update_suggest_text();
    }

    pub fn is_focused(&self) -> bool {
        self.is_active
    }

    pub fn dismiss(&mut self) {
        self.contents.clear();
        self.is_active = false;
    }

    fn find_best_command(&self) -> Option<&Command> {
        if self.contents.is_empty() {
            return None;
        }
        if let Ok(s) = shellwords::split(&self.contents) {
            let ep = s.into_iter().nth(0)?;
            // TODO(feature) edit distance?
            for cmd in &self.commands {
                if cmd.entrypoint.find(&ep).is_some() {
                    return Some(cmd);
                }
            }
        }
        None
    }

    fn get_list_of_entrypoints(&self) -> String {
        self.commands
            .iter()
            .map(|c| format!("{} ", c.entrypoint))
            .collect()
    }

    fn get_args(&self) -> Vec<String> {
        self.contents
            .split_whitespace()
            .skip(1)
            .map(|s| s.to_string())
            .collect()
    }

    pub fn update_suggest_text(&mut self) {
        if let Some(cmd) = self.find_best_command() {
            self.suggest_text = cmd.to_suggestion();
        } else {
            self.suggest_text = self.get_list_of_entrypoints();
        }
    }

    pub fn display_text(&self) -> Vec<(char, bool)> {
        let mut ret = Vec::new();
        let args = self.get_args();
        if !args.is_empty()
            && let Some(cmd) = self.find_best_command()
        {
            for c in cmd.entrypoint.chars() {
                ret.push((c, true));
            }
            let args = cmd.parse_partial_args(&args);
            for (param, arg) in args {
                ret.push((' ', false));
                let param = param.unwrap_or("???".to_string());
                let arg = arg.unwrap_or("_".to_string());
                for c in param.chars() {
                    ret.push((c, false));
                }
                ret.push((':', false));
                ret.push((' ', false));
                for c in arg.chars() {
                    ret.push((c, true));
                }
            }
        } else {
            for i in 0..(self.contents.len().max(self.suggest_text.len())) {
                if let Some(c) = self.contents.chars().nth(i) {
                    ret.push((c, true));
                } else if let Some(c) = self.suggest_text.chars().nth(i) {
                    ret.push((c, false));
                }
            }
        }
        ret
    }

    pub fn fg_text(&self) -> String {
        self.display_text()
            .into_iter()
            .map(|(c, t)| if t { c } else { ' ' })
            .collect()
    }

    pub fn bg_text(&self) -> String {
        self.display_text().into_iter().map(|(c, _)| c).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::cmd::CommandPrompt;

    #[test]
    fn command_prompt() {
        let mut cmd = CommandPrompt::new();
        cmd.focus();

        cmd.on_event(&rdev::Event {
            time: SystemTime::now(),
            name: Some("j".to_string()),
            event_type: rdev::EventType::KeyPress(rdev::Key::KeyJ),
        });

        cmd.on_event(&rdev::Event {
            time: SystemTime::now(),
            name: Some("r".to_string()),
            event_type: rdev::EventType::KeyPress(rdev::Key::KeyR),
        });

        dbg!(cmd.bg_text());
        dbg!(cmd.fg_text());
    }
}
