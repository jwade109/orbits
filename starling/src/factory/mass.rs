#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mass(u64);

impl Mass {
    pub const GRAMS_PER_KILOGRAM: u64 = 1_000;
    pub const GRAMS_PER_TON: u64 = 1_000_000;

    pub fn grams(g: u64) -> Self {
        Mass(g)
    }

    pub fn kilograms(kg: u64) -> Self {
        Mass(kg * 1_000)
    }

    pub fn tons(t: u64) -> Self {
        Mass(t * 1_000_000)
    }

    pub fn to_grams(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for Mass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1000 {
            write!(f, "{} g", self.0)
        } else if self.0 < 1000000 {
            write!(f, "{:0.1} kg", self.0 as f32 / 1000.0)
        } else {
            write!(f, "{:0.1} t", (self.0 / 1000) as f32 / 1000.0)
        }
    }
}
