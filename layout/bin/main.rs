use layout::layout::*;
use layout::svg::write_svg;

fn draw_layout(root: &Node, path: &str) -> Result<(), std::io::Error> {
    let aabbs = do_aabbs(root, (0.0, 0.0).into());
    write_svg(path, &aabbs)
}

fn box_with_corners(w: f32) -> Node {
    let banner = || {
        Node::row(Size::Fit)
            .with_child(Node::new(w, w))
            .with_child(Node::grow())
            .with_child(Node::new(w, w))
    };

    Node::grow()
        .tight()
        .down()
        .with_child(banner())
        .with_child(Node::grow())
        .with_child(banner())
}

fn main() -> Result<(), std::io::Error> {
    let spacing = 4.0;

    let sidebar = Node::column(300.0)
        .with_spacing(spacing)
        .with_children((0..12).map(|i| Node::column(100 + i * 2)))
        .with_child(Node::new(250.0, 100.0))
        .with_children((0..4).map(|_| Node::row(25)));

    let topbar = Node::row(Size::Fit)
        .with_spacing(spacing)
        .with_children((0..10).map(|_| Node::new(40, 20)))
        .with_child(Node::grow())
        .with_child(Node::grow());

    let main = Node::grow().tight().with_child(sidebar).with_child(
        Node::grow()
            .down()
            .tight()
            .with_children([box_with_corners(40.0), Node::row(30)].into_iter()),
    );

    let root = Node::new(1500, 800)
        .tight()
        .down()
        .with_child(topbar)
        .with_child(main);

    let root = populate_fit_sizes(root);
    let root = populate_grow_sizes(root);

    draw_layout(&root, "layout.svg")
}
