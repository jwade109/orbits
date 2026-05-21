use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ParseError {
    BadKey,
    BadValue,
    WrongArgumentCount,
    CommandNotFound,
    NotImplemented,
}

pub type ArgsMap = BTreeMap<String, String>;

pub struct Command<T> {
    pub entrypoint: String,
    pub params: Vec<String>,
    pub func: Box<dyn Fn(&ArgsMap) -> Result<T, ParseError>>,
}

impl<T> Command<T> {
    pub fn new(
        entrypoint: &'static str,
        params: Vec<&'static str>,
        f: impl Fn(&ArgsMap) -> Result<T, ParseError> + 'static,
    ) -> Self {
        Self {
            entrypoint: entrypoint.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            func: Box::new(f),
        }
    }

    pub fn to_suggestion(&self) -> String {
        let mut ret = self.entrypoint.clone();
        for param in &self.params {
            ret += &format!(" [{}]", param);
        }
        ret
    }

    pub fn parse_partial_args(&self, args: &[String]) -> Vec<(Option<String>, Option<String>)> {
        let mut ret = Vec::new();

        for i in 0..self.params.len().max(args.len()) {
            let p = self.params.get(i);
            let a = args.get(i);
            ret.push((p.cloned(), a.cloned()));
        }
        ret
    }

    pub fn parse_complete_args(&self, args: &[String]) -> Option<ArgsMap> {
        let mut ret = ArgsMap::new();
        for (i, param) in self.params.iter().enumerate() {
            let arg = args.get(i)?;
            ret.insert(param.clone(), arg.clone());
        }
        Some(ret)
    }

    pub fn parse(&self, args: &[String]) -> Result<T, ParseError> {
        let args = self
            .parse_complete_args(args)
            .ok_or(ParseError::WrongArgumentCount)?;
        (self.func)(&args)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Error,
}

pub struct Terminal<T> {
    pub contents: String,
    pub is_active: bool,
    pub lines: Vec<(String, LogLevel)>,
    pub suggest_text: String,
    pub commands: Vec<Command<T>>,
}

impl<T> Terminal<T> {
    pub fn with_commands(commands: impl Into<Vec<Command<T>>>) -> Self {
        Self {
            contents: String::new(),
            is_active: false,
            lines: Vec::new(),
            suggest_text: String::new(),
            commands: commands.into(),
        }
    }

    pub fn on_event(&mut self, event: &rdev::Event) -> Option<T> {
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

    fn on_enter(&mut self) -> Option<T> {
        if !self.is_active {
            return None;
        }
        if self.contents.is_empty() {
            return None;
        }

        self.log_info(self.contents.clone());

        let mut ret = None;

        if let Some(cmd) = self.find_best_command() {
            match cmd.parse(&self.get_args()) {
                Ok(action) => {
                    ret = Some(action);
                }
                Err(e) => {
                    self.log_error(format!("Failed to parse: {e:?}"));
                }
            }
        } else {
            self.log_error(format!("Bad command"));
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

    fn find_best_command(&self) -> Option<&Command<T>> {
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

    pub fn log_debug(&mut self, s: String) {
        self.lines.push((s, LogLevel::Debug));
    }

    pub fn log_info(&mut self, s: String) {
        self.lines.push((s, LogLevel::Info));
    }

    pub fn log_error(&mut self, s: String) {
        self.lines.push((s, LogLevel::Error));
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
}
