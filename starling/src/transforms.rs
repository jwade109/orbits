use bevy::math::Vec2;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FrameId(pub i64);

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub pos: Vec2,
    pub vel: Vec2,
    pub angle: f32,
    pub turn_rate: f32,

    pub parent: FrameId,
    pub child: FrameId,
}

impl Transform {
    pub fn identity(parent: FrameId, child: FrameId) -> Self {
        Transform {
            parent,
            child,
            ..Transform::default()
        }
    }

    pub fn from_translation(parent: FrameId, child: FrameId, pos: Vec2) -> Self {
        Transform {
            parent,
            child,
            pos,
            ..Transform::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransformTree {
    pub map: HashMap<(FrameId, FrameId), Transform>,
}

impl TransformTree {
    pub fn update(&mut self, tf: Transform) {
        self.map.insert((tf.parent, tf.child), tf);
    }

    pub fn lookup(&self, src: FrameId, dst: FrameId) -> Option<Transform>
    {
        let path = self.traversal_path(src, dst)?;
        if path.len() < 2
        {
            return Some(Transform::identity(src, dst));
        }

        for i in 0..path.len() - 1
        {
            let fa = path[i];
            let fb = path[i+1];
            if let Some(transform) = self.map.get(&(fa, fb))
            {
                return Some(*transform);
            }
        }

        None
    }

    fn get_parent(&self, child: FrameId) -> Option<FrameId> {
        self.map.keys().find(|(p, c)| *c == child).map(|(p, c)| *p)
    }

    fn get_ancestry(&self, child: FrameId) -> Vec<FrameId> {
        let mut ret = vec![child];
        let mut cur = child;
        while let Some(p) = self.get_parent(cur) {
            ret.push(p);
            cur = p;
        }
        ret
    }

    fn traversal_path(&self, src: FrameId, dst: FrameId) -> Option<Vec<FrameId>> {
        let mut sa = self.get_ancestry(src);
        let mut sb = self.get_ancestry(dst);

        let mut common = None;

        while let (Some(fa), Some(fb)) = (sa.last().cloned(), sb.last().cloned()) {
            if fa == fb {
                common = Some(fa);
            } else {
                break;
            }
            sa.pop();
            sb.pop();
        }

        if let Some(c) = common {
            sa.push(c);
            sb.reverse();
            sa.extend(sb);
            return Some(sa);
        }

        None
    }
}

#[test]
fn get_ancestry() {
    let mut tree = TransformTree::default();

    // 3 -- 4 -- 5
    //  \
    //   +- 6

    tree.update(Transform::identity(FrameId(3), FrameId(4)));
    tree.update(Transform::identity(FrameId(4), FrameId(5)));
    tree.update(Transform::identity(FrameId(3), FrameId(6)));

    assert_eq!(tree.get_ancestry(FrameId(3)), vec![FrameId(3)]);
    assert_eq!(tree.get_ancestry(FrameId(4)), vec![FrameId(4), FrameId(3)]);
    assert_eq!(
        tree.get_ancestry(FrameId(5)),
        vec![FrameId(5), FrameId(4), FrameId(3)]
    );
    assert_eq!(tree.get_ancestry(FrameId(6)), vec![FrameId(6), FrameId(3)]);

    tree.update(Transform::identity(FrameId(2), FrameId(3)));

    assert_eq!(tree.get_ancestry(FrameId(3)), vec![FrameId(3), FrameId(2)]);
    assert_eq!(
        tree.get_ancestry(FrameId(4)),
        vec![FrameId(4), FrameId(3), FrameId(2)]
    );
    assert_eq!(
        tree.get_ancestry(FrameId(5)),
        vec![FrameId(5), FrameId(4), FrameId(3), FrameId(2)]
    );
    assert_eq!(
        tree.get_ancestry(FrameId(6)),
        vec![FrameId(6), FrameId(3), FrameId(2)]
    );
}

#[test]
fn common_ancestors() {
    let mut tree = TransformTree::default();

    // 3 -- 4 -- 7 -- 23
    //            \
    //             +- 13 -- 31 -- 39
    //
    // 19 -- 45 -- 201
    //   \
    //    +- 20 -- 2

    tree.update(Transform::identity(FrameId(3), FrameId(4)));
    tree.update(Transform::identity(FrameId(4), FrameId(7)));
    tree.update(Transform::identity(FrameId(7), FrameId(23)));
    tree.update(Transform::identity(FrameId(7), FrameId(13)));
    tree.update(Transform::identity(FrameId(13), FrameId(31)));
    tree.update(Transform::identity(FrameId(31), FrameId(39)));

    tree.update(Transform::identity(FrameId(19), FrameId(45)));
    tree.update(Transform::identity(FrameId(45), FrameId(201)));
    tree.update(Transform::identity(FrameId(19), FrameId(20)));
    tree.update(Transform::identity(FrameId(20), FrameId(2)));

    assert_eq!(
        tree.traversal_path(FrameId(23), FrameId(39)),
        Some(vec![
            FrameId(23),
            FrameId(7),
            FrameId(13),
            FrameId(31),
            FrameId(39)
        ])
    );

    assert_eq!(tree.get_ancestry(FrameId(4)), vec![FrameId(4), FrameId(3)]);
    assert_eq!(
        tree.get_ancestry(FrameId(23)),
        vec![FrameId(23), FrameId(7), FrameId(4), FrameId(3)]
    );

    assert_eq!(
        tree.traversal_path(FrameId(4), FrameId(23)),
        Some(vec![FrameId(4), FrameId(7), FrameId(23)])
    );

    assert_eq!(
        tree.get_ancestry(FrameId(2)),
        vec![FrameId(2), FrameId(20), FrameId(19)]
    );
    assert_eq!(
        tree.get_ancestry(FrameId(201)),
        vec![FrameId(201), FrameId(45), FrameId(19)]
    );
    assert_eq!(
        tree.traversal_path(FrameId(2), FrameId(201)),
        Some(vec![
            FrameId(2),
            FrameId(20),
            FrameId(19),
            FrameId(45),
            FrameId(201),
        ])
    );

    assert_eq!(tree.traversal_path(FrameId(900), FrameId(45)), None);
    assert_eq!(tree.traversal_path(FrameId(400), FrameId(7)), None);
    assert_eq!(tree.traversal_path(FrameId(205), FrameId(1004)), None);

    assert_eq!(tree.traversal_path(FrameId(7), FrameId(2)), None);

    //       +- 3 -- 4 -- 7 -- 23
    //      /              \
    //     1                +- 13 -- 31 -- 39
    //      \
    //       +- 19 -- 45 -- 201
    //            \
    //             +- 20 -- 2

    tree.update(Transform::identity(FrameId(1), FrameId(3)));
    tree.update(Transform::identity(FrameId(1), FrameId(19)));

    assert_eq!(
        tree.traversal_path(FrameId(7), FrameId(2)),
        Some(vec![
            FrameId(7),
            FrameId(4),
            FrameId(3),
            FrameId(1),
            FrameId(19),
            FrameId(20),
            FrameId(2),
        ])
    );
}
