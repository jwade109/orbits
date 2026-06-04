use bary_ui::*;

fn draw_layout<T: UiMsg>(tree: &Tree<T>, path: &str) -> Result<(), std::io::Error> {
    let aabbs: Vec<_> = tree
        .layouts()
        .iter()
        .map(|n| (n.aabb(), n.color()))
        .collect();
    write_svg(path, &aabbs)
}

fn main() -> Result<(), std::io::Error> {
    let tree = example_layout(1300.0, 800.0);
    draw_layout(&tree, "bary_ui.svg")
}
