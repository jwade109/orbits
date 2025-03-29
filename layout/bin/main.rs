use layout::examples::*;
use layout::layout::*;
use layout::svg::write_svg;
use starling::aabb::AABB;
use starling::prelude::Vec2;

fn draw_layout(tree: &Tree, path: &str) -> Result<(), std::io::Error> {
    let visitor = |n: &Node| -> Option<AABB> { n.is_visible().then(|| n.aabb()) };
    let mut aabbs = vec![];
    for root in tree.layouts() {
        aabbs.extend(root.visit(&visitor));
    }
    write_svg(path, &aabbs)
}

fn main() -> Result<(), std::io::Error> {
    let tree = example_layout(1300.0, 800.0);
    draw_layout(&tree, "layout.svg")
}
