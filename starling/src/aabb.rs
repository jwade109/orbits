use bevy::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct AABB(pub Vec2, pub Vec2);

impl AABB {
    pub fn from_center(c: Vec2, span: Vec2) -> Self {
        let low = c - span / 2.0;
        let hi = c + span / 2.0;
        Self(low, hi)
    }

    pub fn from_arbitrary(a: Vec2, b: Vec2) -> Self {
        let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        Self(low, hi)
    }

    pub fn from_list(plist: &[Vec2]) -> Option<Self> {
        let p0 = plist.get(0)?;
        let mut ret = AABB(*p0, *p0);
        for p in plist {
            ret.include(*p)
        }
        Some(ret)
    }

    pub fn padded(&self, padding: f32) -> Self {
        let d = Vec2::new(padding, padding);
        AABB(self.0 - d, self.1 + d)
    }

    pub fn include(&mut self, p: Vec2) {
        self.0.x = self.0.x.min(p.x);
        self.0.y = self.0.y.min(p.y);
        self.1.x = self.1.x.max(p.x);
        self.1.y = self.1.y.max(p.y);
    }

    pub fn center(&self) -> Vec2 {
        (self.0 + self.1) / 2.0
    }

    pub fn span(&self) -> Vec2 {
        self.1 - self.0
    }

    pub fn to_normalized(&self, p: Vec2) -> Vec2 {
        let u = p - self.0;
        let s = self.span();
        u / s
    }

    pub fn from_normalized(&self, u: Vec2) -> Vec2 {
        u * self.span() + self.0
    }

    pub fn map(from: Self, to: Self, p: Vec2) -> Vec2 {
        let u = from.to_normalized(p);
        to.from_normalized(u)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let u = self.to_normalized(p);
        0.0 <= u.x && u.x <= 1.0 && 0.0 <= u.y && u.y <= 1.0
    }
}
