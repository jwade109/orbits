use crate::layout::*;
use starling::prelude::{Vec2, AABB};

fn box_with_corners(w: f32) -> Node {
    let banner = || {
        Node::row(Size::Fit)
            .invisible()
            .with_child(Node::new(w, w))
            .with_child(Node::grow().invisible())
            .with_child(Node::new(w, w))
            .with_child(Node::grow().invisible())
            .with_child(Node::new(w, w))
    };

    Node::grow()
        .invisible()
        .tight()
        .down()
        .with_child(banner())
        .with_child(Node::grow().invisible())
        .with_child(banner())
}

pub fn context_menu(pos: Vec2) -> Tree {
    let spacing = 4.0;
    let window = Node::new(200, 300)
        .down()
        .with_child(Node::row(20))
        .with_child(Node::row(40))
        .with_child(Node::grid(Size::Grow, Size::Grow, 6, 6, spacing));

    Tree::new().with_layout(window, pos)
}

pub fn example_layout(width: f32, height: f32) -> Tree {
    let spacing = 4.0;

    let sidebar = Node::column(300.0)
        .with_spacing(spacing)
        .with_children((0..12).map(|i| Node::column(100 + i * 2)))
        .with_child(Node::grid(Size::Grow, 100.0, 4, 5, spacing).with_child(Node::grow()))
        .with_children((0..4).map(|_| Node::row(25)));

    let topbar = Node::row(Size::Fit)
        .with_spacing(spacing)
        .with_children((0..10).map(|_| Node::new(40, 20)))
        .with_children((0..5).map(|_| Node::grow()))
        .with_child(Node::column(30));

    let main = Node::grow().tight().with_child(sidebar).with_child(
        Node::grow()
            .down()
            .tight()
            .with_children([box_with_corners(40.0), Node::row(30)].into_iter()),
    );

    let root = Node::new(width, height)
        .tight()
        .down()
        .with_child(topbar)
        .with_child(main);

    Tree::new().with_layout(root, None)
}
