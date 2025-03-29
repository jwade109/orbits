use layout::layout::*;
use layout::svg::write_svg;

fn draw_layout(root: &Node, path: &str) -> Result<(), std::io::Error> {
    let aabbs = do_aabbs(root, (0.0, 0.0).into());
    write_svg(path, &aabbs)
}

fn grow(root: Node) -> Node {
    root
}

fn main() -> Result<(), std::io::Error> {
    let sidebar = Node::new(300.0, 700.0)
        .with_layout(LayoutDir::TopToBottom)
        .with_child_gap(3.0)
        .with_children((0..12).map(|i| Node::new(30 + i * 2, 20)))
        .with_child(Node::new(250.0, 100.0))
        .with_children((0..4).map(|_| Node::new(Size::Grow, 25)));

    let topbar = Node::new(Size::Fit, Size::Fit)
        .with_padding(2.0)
        .with_children((0..10).map(|_| Node::new(40, 20)));

    let b = Node::new(Size::Fit, 200.0).with_child(Node::new(300, 40));
    let c = Node::new(550.0, Size::Fit).with_child(Node::new(60, 600));
    let d = Node::new(600.0, 100.0);

    let main = Node::new(Size::Fit, Size::Fit)
        .with_child(sidebar)
        .with_child(b)
        .with_child(c)
        .with_child(d);

    let root = Node::new(Size::Fit, Size::Fit)
        .with_layout(LayoutDir::TopToBottom)
        .with_child(topbar)
        .with_child(main);

    let root = grow(root);

    draw_layout(&root, "layout.svg")
}
