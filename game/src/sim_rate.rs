use enum_iterator::{all, Sequence};

#[derive(Debug, Clone, Copy, Sequence)]
pub enum SimRate {
    RealTime,
    TenSecondsPerSecond,
    MinutePerSecond,
    HourPerSecond,
    DayPerSecond,
    WeekPerSecond,
    MonthPerSecond,
}

impl SimRate {
    pub fn as_str(&self) -> &'static str {
        match self {
            SimRate::RealTime => "1x",
            SimRate::TenSecondsPerSecond => "10x",
            SimRate::MinutePerSecond => "Mix",
            SimRate::HourPerSecond => "Hrx",
            SimRate::DayPerSecond => "Dyx",
            SimRate::WeekPerSecond => "Wkx",
            SimRate::MonthPerSecond => "Mox",
        }
    }

    pub fn as_ticks(&self) -> u32 {
        match self {
            SimRate::RealTime => 1,
            SimRate::TenSecondsPerSecond => 10,
            SimRate::MinutePerSecond => 60,
            SimRate::HourPerSecond => 3600,
            SimRate::DayPerSecond => 3600 * 24,
            SimRate::WeekPerSecond => 3600 * 24 * 7,
            SimRate::MonthPerSecond => 3600 * 24 * 30,
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        all::<Self>()
    }
}
