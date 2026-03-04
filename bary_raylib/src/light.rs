use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct DutyCycle {
    /// number of ticks spent high
    pub on: u32,
    /// number of ticks spent low
    pub off: u32,
    /// time delay of the cycle
    pub delay: u32,
}

impl DutyCycle {
    pub fn new(on: u32, total: u32, delay: u32) -> Self {
        let on = on.min(total);
        let off = total - on;
        let delay = delay % total;
        Self { on, off, delay }
    }

    pub fn is_on(&self, t: u32) -> bool {
        let total = self.on + self.off;
        let t = if t < self.delay { t + total } else { t };
        let t = (t - self.delay) % total;
        t < self.on
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Light {
    pub cycle: DutyCycle,
    pub ticks: u32,
    pub grid_id: Ent,
    pub prototype_id: Ent,
    pub position: Vec2,
}

impl Light {
    pub fn new(grid_id: Ent, prototype_id: Ent, pos: Vec2, idx: u32) -> Self {
        let total = 600;
        let on = 150;
        let delay = idx * 50;
        Self {
            cycle: DutyCycle::new(on, total, delay),
            ticks: 0,
            grid_id,
            prototype_id,
            position: pos,
        }
    }

    pub fn is_on(&self) -> bool {
        self.cycle.is_on(self.ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duty_cycle_1() {
        let cycle = DutyCycle::new(5, 12, 16);

        // HI          @ @ @ @ @
        // LO  @ @ @ @           @ @ @
        //     * * * * * * * * * * * *

        assert_eq!(cycle.on, 5);
        assert_eq!(cycle.off, 7);
        assert_eq!(cycle.delay, 4);

        assert!(!cycle.is_on(0));
        assert!(!cycle.is_on(1));
        assert!(!cycle.is_on(2));
        assert!(!cycle.is_on(3));
        assert!(cycle.is_on(4));
        assert!(cycle.is_on(5));
        assert!(cycle.is_on(6));
        assert!(cycle.is_on(7));
        assert!(cycle.is_on(8));
        assert!(!cycle.is_on(9));
        assert!(!cycle.is_on(10));
        assert!(!cycle.is_on(11));
        assert!(!cycle.is_on(12));
        assert!(!cycle.is_on(13));
        assert!(!cycle.is_on(14));
        assert!(!cycle.is_on(15));
        assert!(cycle.is_on(16));
        assert!(cycle.is_on(17));
    }

    #[test]
    fn duty_cycle_2() {
        let cycle = DutyCycle::new(5, 4, 3);

        assert_eq!(cycle.on, 4);
        assert_eq!(cycle.off, 0);
        assert_eq!(cycle.delay, 3);

        assert!(cycle.is_on(0));
        assert!(cycle.is_on(1));
        assert!(cycle.is_on(2));
        assert!(cycle.is_on(3));
        assert!(cycle.is_on(4));
        assert!(cycle.is_on(5));
        assert!(cycle.is_on(6));
        assert!(cycle.is_on(7));
    }
}
