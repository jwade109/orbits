use bary_core::prelude::{AABB, Vec2};
use bary_ui::{
    examples::example_layout,
    layout::{Node, Size, Tree},
};
use clap::Parser;
use raylib::prelude::*;
use serde::Deserialize;
use std::time::{Duration, SystemTime};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    path: String,
}

fn get_edit_time(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn get_file_contents(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

struct HotReload {
    path: String,
    time: Option<SystemTime>,
    contents: Option<String>,
}

#[derive(Deserialize, Debug)]
struct UiDecl {
    title: String,
    padding: f32,
    nodes: Vec<DeclNode>,
}

#[derive(Deserialize, Debug)]
enum DeclNode {
    Title(String),
    Leaf,
    Image,
    Button(String),
    Text(String),
    Column(Vec<DeclNode>),
    Row(Vec<DeclNode>),
}

impl HotReload {
    fn update(&mut self) {
        let new_time = get_edit_time(&self.path);

        let t1 = new_time.unwrap_or(SystemTime::UNIX_EPOCH);
        let t0 = self.time.unwrap_or(SystemTime::UNIX_EPOCH);

        let dt = t1.duration_since(t0).unwrap_or(Duration::ZERO);

        let is_empty = self.contents.as_ref().map(|c| c.is_empty()).unwrap_or(true);

        if dt > Duration::from_millis(50) || is_empty {
            println!("Reloaded {}", self.path);
            if let Some(c) = get_file_contents(&self.path) {
                self.contents = Some(c);
                self.time = new_time;
            }
        }
    }
}

fn draw_rect(d: &mut RaylibDrawHandle, bounds: AABB, fill: bool) {
    let min = bounds.lower();
    let dims = bounds.span;
    let rec = Rectangle::new(min.x, min.y, dims.x, dims.y);
    let color = Color::WHITE.alpha(0.2);
    if fill {
        d.draw_rectangle_rec(rec, color);
    } else {
        d.draw_rectangle_lines_ex(rec, 1.0, color);
    }
}

fn draw_node(d: &mut RaylibDrawHandle, node: &Node<String>) {
    let aabb = node.aabb();

    draw_rect(d, aabb, false);

    for c in node.children() {
        draw_node(d, c);
    }
}

fn draw_layout(d: &mut RaylibDrawHandle, layout: &Tree<String>) {
    for node in layout.layouts() {
        draw_node(d, node);
    }
}

fn to_node(decl: DeclNode, padding: f32) -> Node<String> {
    match decl {
        DeclNode::Title(s) => Node::text(Size::Grow, 50, s),
        DeclNode::Leaf => Node::text(Size::Grow, Size::Grow, "Leaf"),
        DeclNode::Image => Node::text(150, 150, "Image"),
        DeclNode::Button(_) => Node::text(90, 40, "Button"),
        DeclNode::Text(_) => Node::text(150, 40, "Text"),
        DeclNode::Column(nodes) => {
            let nodes = nodes.into_iter().map(|n| to_node(n, padding));
            Node::column(Size::Fit).with_children(nodes)
        }
        DeclNode::Row(nodes) => {
            let nodes = nodes.into_iter().map(|n| to_node(n, padding));
            Node::row(Size::Fit).with_children(nodes)
        }
    }
}

fn generate_layout(decl: UiDecl, width: f32, height: f32) -> Tree<String> {
    let mut root = Node::structural(width, height);
    for n in decl.nodes {
        let node = to_node(n, decl.padding);
        root.add_child(node);
    }
    return Tree::new().with_layout(root, Vec2::ZERO);
}

fn main() {
    let args = Args::parse();

    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .msaa_4x()
        .resizable()
        .build();

    let mut reload = HotReload {
        path: args.path,
        time: None,
        contents: None,
    };

    while !rl.window_should_close() {
        reload.update();

        let width = rl.get_render_width() as f32;
        let height = rl.get_render_height() as f32;
        // let layout = example_layout(width, height);

        let mut d = rl.begin_drawing(&thread);

        let contents = reload.contents.clone().unwrap_or(String::new());

        if let Ok(decl) = serde_yaml::from_str::<UiDecl>(&contents) {
            let s = format!("{}: {:?}\n{:#?}", reload.path, reload.time, decl);
            d.draw_text(&s, 10, 10, 14, Color::WHITE.alpha(0.1));

            let layout = generate_layout(decl, width, height);

            draw_layout(&mut d, &layout);
        }

        d.clear_background(Color::BLACK);
    }
}
