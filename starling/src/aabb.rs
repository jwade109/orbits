use crate::core::rotate;
use bevy::math::Vec2;

#[derive(Default, Debug, Clone, Copy)]
pub struct AABB {
    pub center: Vec2,
    pub span: Vec2,
}

impl AABB {
    pub fn new(center: Vec2, span: Vec2) -> Self {
        Self { center, span }
    }

    pub fn from_arbitrary(a: impl Into<Vec2>, b: impl Into<Vec2>) -> Self {
        let a: Vec2 = a.into();
        let b: Vec2 = b.into();
        let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        AABB::new((hi + low) / 2.0, hi - low)
    }

    pub fn from_list(plist: &[Vec2]) -> Option<Self> {
        let p0 = plist.get(0)?;
        let mut ret = AABB::new(*p0, Vec2::ZERO);
        for p in plist {
            ret.include(*p)
        }
        Some(ret)
    }

    pub fn from_wh(w: f32, h: f32) -> AABB {
        let span = Vec2::new(w, h);
        AABB::new(Vec2::ZERO, span)
    }

    pub fn padded(&self, padding: f32) -> Self {
        let d = Vec2::new(padding, padding) / 2.0;
        AABB::new(self.center, self.span + d)
    }

    pub fn include(&mut self, p: Vec2) {
        let mut min = self.center - self.span / 2.0;
        let mut max = self.center + self.span / 2.0;
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        self.center = (max + min) / 2.0;
        self.span = max - min;
    }

    pub fn corners(&self) -> [Vec2; 4] {
        let c = self.center;
        let ur = self.span / 2.0;
        let ul = Vec2::new(-ur.x, ur.y);
        let ll = Vec2::new(-ur.x, -ur.y);
        let lr = Vec2::new(ur.x, -ur.y);
        [c + ur, c + lr, c + ll, c + ul]
    }

    pub fn offset(&self, d: Vec2) -> Self {
        AABB::new(self.center + d, self.span)
    }

    pub fn to_normalized(&self, p: Vec2) -> Vec2 {
        let lower = self.center - self.span / 2.0;
        let u = p - lower;
        u / self.span
    }

    pub fn from_normalized(&self, u: Vec2) -> Vec2 {
        let lower = self.center - self.span / 2.0;
        u * self.span + lower
    }

    pub fn map(from: Self, to: Self, p: Vec2) -> Vec2 {
        let u = from.to_normalized(p);
        to.from_normalized(u)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let u = self.to_normalized(p);
        0.0 <= u.x && u.x <= 1.0 && 0.0 <= u.y && u.y <= 1.0
    }

    pub fn scale(&self, scalar: f32) -> Self {
        AABB::new(self.center, self.span * scalar)
    }
}

impl From<((f32, f32), (f32, f32))> for AABB {
    fn from(value: ((f32, f32), (f32, f32))) -> Self {
        AABB::from_arbitrary(value.0, value.1)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OBB(AABB, f32);

impl OBB {
    pub fn new(aabb: impl Into<AABB>, angle: f32) -> Self {
        OBB(aabb.into(), angle)
    }

    pub fn offset(&self, d: Vec2) -> Self {
        OBB(self.0.offset(d), self.1)
    }

    pub fn corners(&self) -> [Vec2; 4] {
        let c = self.0.center;
        let mut corners = self.0.corners();

        for p in &mut corners {
            *p = rotate(*p - c, self.1) + c;
        }

        corners
    }
}
