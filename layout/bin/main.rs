use layout::examples::*;
use layout::layout::*;
use layout::svg::write_svg;
use starling::aabb::AABB;

fn draw_layout(tree: &Tree<String>, path: &str) -> Result<(), std::io::Error> {
    let aabbs: Vec<AABB> = tree.layouts().iter().map(|n| n.aabb()).collect();
    write_svg(path, &aabbs)
}

fn main() -> Result<(), std::io::Error> {
    let tree = example_layout(1300.0, 800.0);
    draw_layout(&tree, "layout.svg")
}
