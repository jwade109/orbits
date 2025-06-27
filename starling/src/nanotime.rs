use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Nanotime(i64);

impl Nanotime {
    pub const PER_MILLI: i64 = 1000000;
    pub const PER_SEC: i64 = Nanotime::PER_MILLI * 1000;
    pub const PER_MINUTE: i64 = Nanotime::PER_SEC * 60;
    pub const PER_HOUR: i64 = Nanotime::PER_MINUTE * 60;
    pub const PER_DAY: i64 = Nanotime::PER_HOUR * 24;
    pub const PER_WEEK: i64 = Nanotime::PER_DAY * 7;
    pub const PER_YEAR: i64 = Nanotime::PER_DAY * 365;

    pub fn zero() -> Self {
        Nanotime(0)
    }

    pub fn to_secs(&self) -> f32 {
        self.0 as f32 / Nanotime::PER_SEC as f32
    }

    pub fn to_secs_f64(&self) -> f64 {
        self.0 as f64 / Nanotime::PER_SEC as f64
    }

    pub fn to_parts(&self) -> (i64, i64) {
        (self.0 % Nanotime::PER_SEC, self.0 / Nanotime::PER_SEC)
    }

    pub fn nanos(ns: i64) -> Self {
        Nanotime(ns)
    }

    pub fn secs(s: i64) -> Self {
        Nanotime(s * Nanotime::PER_SEC)
    }

    pub fn mins(m: i64) -> Self {
        Nanotime(m * Nanotime::PER_MINUTE)
    }

    pub fn hours(h: i64) -> Self {
        Nanotime(h * Nanotime::PER_HOUR)
    }

    pub fn days(d: i64) -> Self {
        Nanotime(d * Nanotime::PER_DAY)
    }

    pub const fn millis(ms: i64) -> Self {
        Nanotime(ms * Nanotime::PER_MILLI)
    }

    pub fn secs_f32(s: f32) -> Self {
        Nanotime((s * Nanotime::PER_SEC as f32) as i64)
    }

    pub fn secs_f64(s: f64) -> Self {
        Nanotime((s * Nanotime::PER_SEC as f64) as i64)
    }

    pub fn ceil(&self, order: i64) -> Self {
        Self((self.0 + order) - (self.0 % order))
    }

    pub fn floor(&self, order: i64) -> Self {
        Self(self.0 - (self.0 % order))
    }

    pub fn inner(&self) -> i64 {
        self.0
    }

    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub fn lerp(self, other: Self, s: f32) -> Self {
        if s == 0.0 {
            self
        } else if s == 1.0 {
            other
        } else {
            self + (other - self) * s
        }
    }

    pub fn to_date(&self) -> Date {
        let div = |rem: i64, denom: i64| (rem / denom, rem % denom);

        let (year, rem) = div(self.0, Nanotime::PER_YEAR);
        let (week, rem) = div(rem, Nanotime::PER_WEEK);
        let (day, rem) = div(rem, Nanotime::PER_DAY);
        let (hour, rem) = div(rem, Nanotime::PER_HOUR);
        let (min, rem) = div(rem, Nanotime::PER_MINUTE);
        let (sec, rem) = div(rem, Nanotime::PER_SEC);
        let (milli, _) = div(rem, Nanotime::PER_MILLI);
        Date {
            year,
            week,
            day,
            hour,
            min,
            sec,
            milli,
        }
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Y{} W{} D{} {:02}:{:02}:{:02}.{:03}",
            self.year + 1,
            self.week + 1,
            self.day + 1,
            self.hour,
            self.min,
            self.sec,
            self.milli,
        )
    }
}

fn fmt(s: &Nanotime, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let disp = s.0.abs();
    if s.0 >= 0 {
        write!(f, "{}.{:09}", disp / 1000000000, disp % 1000000000)
    } else {
        write!(f, "-{}.{:09}", disp / 1000000000, disp % 1000000000)
    }
}

impl core::fmt::Debug for Nanotime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt(self, f)
    }
}

impl std::fmt::Display for Nanotime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt(self, f)
    }
}

impl Add for Nanotime {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Nanotime(self.0 + other.0)
    }
}

impl AddAssign for Nanotime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub for Nanotime {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        // TODO disallow wrapping?
        Nanotime(self.0.wrapping_sub(other.0))
    }
}

impl SubAssign for Nanotime {
    fn sub_assign(&mut self, rhs: Self) {
        let res = self.sub(rhs);
        *self = res;
    }
}

impl Mul<i64> for Nanotime {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Self(self.0 * rhs)
    }
}

impl Mul<f32> for Nanotime {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self((self.0 as f32 * rhs) as i64)
    }
}

impl Div<i64> for Nanotime {
    type Output = Self;
    fn div(self, rhs: i64) -> Self {
        Self(self.0 / rhs)
    }
}

impl Rem<Nanotime> for Nanotime {
    type Output = Self;
    fn rem(self, rhs: Nanotime) -> Self::Output {
        Nanotime(self.0 % rhs.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Date {
    pub year: i64,
    pub week: i64,
    pub day: i64,
    pub hour: i64,
    pub min: i64,
    pub sec: i64,
    pub milli: i64,
}
