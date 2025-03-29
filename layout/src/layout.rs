#![allow(dead_code)]
#![allow(unused)]

use starling::aabb::AABB;
use starling::prelude::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum LayoutDir {
    LeftToRight,
    TopToBottom,
}

#[derive(Debug)]
pub enum Size {
    Grow,
    Fit,
    Fixed(f32),
}

impl Size {
    fn as_fixed(&self) -> f32 {
        if let Size::Fixed(s) = self {
            *s
        } else {
            0.0
        }
    }
}

impl Into<Size> for f32 {
    fn into(self) -> Size {
        Size::Fixed(self)
    }
}

impl Into<Size> for u32 {
    fn into(self) -> Size {
        Size::Fixed(self as f32)
    }
}

#[derive(Debug)]
pub struct Node {
    width: Size,
    height: Size,
    layout: LayoutDir,
    children: Vec<Node>,
    child_gap: f32,
    padding: f32,
}

impl Node {
    pub fn new(width: impl Into<Size>, height: impl Into<Size>) -> Self {
        Node {
            width: width.into(),
            height: height.into(),
            layout: LayoutDir::LeftToRight,
            children: Vec::new(),
            child_gap: 10.0,
            padding: 10.0,
        }
    }

    pub fn with_layout(mut self, layout: LayoutDir) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_child_gap(mut self, child_gap: f32) -> Self {
        self.child_gap = child_gap;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_child(mut self, n: Node) -> Self {
        self.add_child(n);
        self
    }

    pub fn with_children(mut self, nodes: impl Iterator<Item = Node>) -> Self {
        nodes.for_each(|n| {
            self.add_child(n);
        });
        self
    }

    pub fn children(&self) -> impl Iterator<Item = &Node> + use<'_> {
        self.children.iter()
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn add_child(&mut self, n: Node) -> &mut Self {
        self.children.push(n);
        self
    }

    pub fn fixed_dims(&self) -> Vec2 {
        if self.is_leaf() {
            return Vec2::new(self.width.as_fixed(), self.height.as_fixed());
        }

        let dims = sum_fixed_dims(self.layout, self.children(), self.padding, self.child_gap);

        let dx = match self.width {
            Size::Fit => dims.x,
            Size::Grow => dims.x,
            Size::Fixed(x) => x,
        };

        let dy = match self.height {
            Size::Fit => dims.y,
            Size::Grow => dims.y,
            Size::Fixed(y) => y,
        };

        Vec2::new(dx, dy)
    }
}

struct Tree {
    root: Node,
}

fn sum_fixed_dims<'a>(
    layout: LayoutDir,
    nodes: impl Iterator<Item = &'a Node>,
    padding: f32,
    childgap: f32,
) -> Vec2 {
    let mut sx: f32 = 0.0;
    let mut sy: f32 = 0.0;

    for node in nodes {
        let dims = node.fixed_dims();
        match layout {
            LayoutDir::LeftToRight => {
                sx += dims.x + childgap;
                sy = sy.max(dims.y);
            }
            LayoutDir::TopToBottom => {
                sx = sx.max(dims.x);
                sy += dims.y + childgap;
            }
        };
    }

    if sx > 0.0 {
        match layout {
            LayoutDir::LeftToRight => sx -= childgap,
            LayoutDir::TopToBottom => sy -= childgap,
        }
    }

    sx += padding * 2.0;
    sy += padding * 2.0;

    Vec2::new(sx, sy)
}

pub fn do_aabbs<'a>(root: &Node, origin: Vec2) -> Vec<AABB> {
    let mut px = origin.x + root.padding;
    let mut py = origin.y + root.padding;

    let mut ret = vec![AABB::from_arbitrary(origin, origin + root.fixed_dims())];

    for node in root.children() {
        let dim = node.fixed_dims();
        // let aabb = AABB::from_arbitrary((px, py), (px + dim.x, py + dim.y));
        // ret.push(aabb);
        let children = do_aabbs(node, (px, py).into());
        ret.extend_from_slice(&children);

        match root.layout {
            LayoutDir::LeftToRight => px += dim.x + root.child_gap,
            LayoutDir::TopToBottom => py += dim.y + root.child_gap,
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_dims() {
        let a = Node::new(300.0, 700.0);
        let b = Node::new(200.0, 400.0);
        let c = Node::new(550.0, 300.0);

        let nodes = [&a, &b, &c];

        let l2r = sum_fixed_dims(LayoutDir::LeftToRight, nodes.into_iter(), 0.0, 0.0);
        let t2b = sum_fixed_dims(LayoutDir::TopToBottom, nodes.into_iter(), 0.0, 0.0);

        assert_eq!(l2r.x, 1050.0);
        assert_eq!(l2r.y, 700.0);

        assert_eq!(t2b.x, 550.0);
        assert_eq!(t2b.y, 1400.0);

        let l2r = sum_fixed_dims(LayoutDir::LeftToRight, nodes.into_iter(), 12.0, 7.5);
        let t2b = sum_fixed_dims(LayoutDir::TopToBottom, nodes.into_iter(), 12.0, 7.5);

        assert_eq!(l2r.x, 1089.0);
        assert_eq!(l2r.y, 724.0);

        assert_eq!(t2b.x, 574.0);
        assert_eq!(t2b.y, 1439.0);

        let root = Node::new(Size::Fit, Size::Fit)
            .with_child(a)
            .with_child(b)
            .with_child(c);

        let dims = root.fixed_dims();

        assert_eq!(dims.x, 1080.0);
        assert_eq!(dims.y, 720.0);
    }
}
