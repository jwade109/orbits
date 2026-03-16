use super::commands::*;
use crate::assets::Assets;
use crate::multiplayer::Action;
use bary_core::prelude::*;
use raylib::prelude::*;
use std::collections::VecDeque;

pub struct Command {
    entrypoint: String,
    params: Vec<String>,
    func: Box<dyn Fn(&ArgsMap) -> Result<Action, ParseError>>,
}

impl Command {
    fn new(
        entrypoint: &'static str,
        params: Vec<&'static str>,
        f: impl Fn(&ArgsMap) -> Result<Action, ParseError> + 'static,
    ) -> Self {
        Self {
            entrypoint: entrypoint.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            func: Box::new(f),
        }
    }

    fn to_suggestion(&self) -> String {
        let mut ret = self.entrypoint.clone();
        for param in &self.params {
            ret += &format!(" [{}]", param);
        }
        ret
    }

    fn parse_partial_args(&self, args: &[String]) -> Vec<(Option<String>, Option<String>)> {
        let mut ret = Vec::new();

        for i in 0..self.params.len().max(args.len()) {
            let p = self.params.get(i);
            let a = args.get(i);
            ret.push((p.cloned(), a.cloned()));
        }
        ret
    }

    fn parse_complete_args(&self, args: &[String]) -> Option<ArgsMap> {
        let mut ret = ArgsMap::new();
        for (i, param) in self.params.iter().enumerate() {
            let arg = args.get(i)?;
            ret.insert(param.clone(), arg.clone());
        }
        Some(ret)
    }

    fn parse(&self, args: &[String]) -> Result<Action, ParseError> {
        let args = self
            .parse_complete_args(args)
            .ok_or(ParseError::WrongArgumentCount)?;
        (self.func)(&args)
    }
}

enum Severity {
    Info,
    Error,
}

impl Severity {
    fn color(&self) -> Color {
        match self {
            Self::Error => Color::RED,
            Self::Info => Color::RAYWHITE,
        }
    }
}

pub struct CommandPrompt {
    contents: String,
    is_active: bool,
    queued_commands: VecDeque<Action>,
    lines: Vec<(String, Severity)>,
    suggest_text: String,
    commands: Vec<Command>,
}

impl CommandPrompt {
    pub fn new() -> Self {
        Self {
            contents: String::new(),
            is_active: false,
            queued_commands: VecDeque::new(),
            lines: Vec::new(),
            suggest_text: String::new(),
            commands: vec![
                Command::new("goto", vec!["grid_name"], cmd_goto),
                Command::new("spawn", vec!["bp_name", "x", "y"], cmd_spawn),
                Command::new("edit", vec!["grid_id"], cmd_edit),
                Command::new("despawn", vec!["grid_id"], cmd_despawn),
                Command::new("find", vec!["grid_name"], cmd_find),
                Command::new("ping", vec!["x", "y"], cmd_ping),
                Command::new("waypoint", vec!["grid_id", "x", "y"], cmd_waypoint),
                Command::new("setspeed", vec!["speed"], cmd_set_speed),
                Command::new("save", vec![], cmd_placeholder),
                Command::new("exit", vec![], |_args| panic!()),
            ],
        }
    }

    pub fn on_backspace(&mut self) {
        if !self.is_active {
            return;
        }
        self.contents.pop();
        self.update_suggest_text();
    }

    fn info(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.lines.push((s, Severity::Info));
    }

    fn error(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.lines.push((s, Severity::Error));
    }

    fn on_enter(&mut self) {
        if !self.is_active {
            return;
        }
        if self.contents.is_empty() {
            return;
        }

        self.lines.push((self.contents.clone(), Severity::Info));

        if let Some(cmd) = self.find_best_command() {
            match cmd.parse(&self.get_args()) {
                Ok(action) => {
                    self.queued_commands.push_back(action);
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
        let s = self.contents.split(" ");
        let ep = s.into_iter().nth(0)?;
        // TODO(feature) edit distance?
        for cmd in &self.commands {
            if cmd.entrypoint.find(ep).is_some() {
                return Some(cmd);
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

    pub fn pop_action(&mut self) -> Option<Action> {
        self.queued_commands.pop_front()
    }
}

pub fn cmd_handle_input_event(cmd: &mut CommandPrompt, event: &rdev::Event) {
    if let rdev::EventType::KeyPress(k) = &event.event_type {
        match k {
            rdev::Key::Backspace => cmd.on_backspace(),
            rdev::Key::Return => cmd.on_enter(),
            rdev::Key::BackQuote => cmd.focus(),
            rdev::Key::Escape => cmd.dismiss(),
            rdev::Key::Tab => cmd.on_tab_complete(),
            _ => {
                if let Some(n) = &event.name {
                    if n.is_ascii() {
                        cmd.append(n);
                    }
                }
            }
        }
    }
}

pub fn draw_command_prompt(d: &mut RaylibDrawHandle, cmd: &CommandPrompt, assets: &Assets) {
    if !cmd.is_active {
        return;
    };

    let Some(font) = &assets.fira_code else {
        return;
    };

    // let cursor = if d.get_time() % 1.2 > 0.6 { "_" } else { "" };

    let chars = cmd.display_text();

    let fg: String = chars
        .iter()
        .map(|(c, b)| if *b { *c } else { ' ' })
        .collect();

    let bg: String = chars
        .iter()
        .map(|(c, b)| if !*b { *c } else { ' ' })
        .collect();

    let display = format!("> {}", fg);
    let display_bg = format!("> {}", bg);
    let width = d.get_render_width();
    let height = d.get_render_height();

    let padding = 30;
    let line_gap = 10;
    let font_size = 52;

    let rect_height = font_size + padding * 2 + (font_size + line_gap) * cmd.lines.len() as i32;
    let rect_origin = IVec2::new(0, height - rect_height);
    let text_origin = IVec2::new(padding, height - padding - font_size);

    {
        d.draw_rectangle(
            rect_origin.x,
            rect_origin.y,
            width,
            height,
            Color::new(10, 10, 30, 220),
        );
    }

    for (i, (line, severity)) in cmd.lines.iter().rev().enumerate() {
        let origin = text_origin - IVec2::Y * (font_size + line_gap) * (i as i32 + 1);
        d.draw_text_ex(
            font,
            &line,
            Vector2::new(origin.x as f32, origin.y as f32),
            font_size as f32,
            1.0,
            severity.color(),
        );
    }

    {
        d.draw_text_ex(
            font,
            &display_bg,
            Vector2::new(text_origin.x as f32, text_origin.y as f32),
            font_size as f32,
            1.0,
            Color::GRAY.alpha(0.6),
        );

        d.draw_text_ex(
            font,
            &display,
            Vector2::new(text_origin.x as f32, text_origin.y as f32),
            font_size as f32,
            1.0,
            Color::WHITE,
        );
    }
}
