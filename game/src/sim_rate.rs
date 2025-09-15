use enum_iterator::{all, Sequence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum SimRate {
    RealTime,
    ThreeSecondsPerSecond,
    TenSecondsPerSecond,
    MinutePerSecond,
    FiveMinsPerSecond,
    FifteenMinsPerSecond,
    ThirtyMinsPerSecond,
    HourPerSecond,
    ThreeHourPerSecond,
    SixHourPerSecond,
    TwelveHourPerSecond,
    DayPerSecond,
    WeekPerSecond,
    MonthPerSecond,
}

impl SimRate {
    pub fn as_str(&self) -> &'static str {
        match self {
            SimRate::RealTime => "1s",
            SimRate::ThreeSecondsPerSecond => "3s",
            SimRate::TenSecondsPerSecond => "10s",
            SimRate::MinutePerSecond => "1m",
            SimRate::FiveMinsPerSecond => "5m",
            SimRate::FifteenMinsPerSecond => "15m",
            SimRate::ThirtyMinsPerSecond => "30m",
            SimRate::HourPerSecond => "Hr",
            SimRate::ThreeHourPerSecond => "3Hr",
            SimRate::SixHourPerSecond => "6Hr",
            SimRate::TwelveHourPerSecond => "12Hr",
            SimRate::DayPerSecond => "Dy",
            SimRate::WeekPerSecond => "Wk",
            SimRate::MonthPerSecond => "Mn",
        }
    }

    pub fn as_ticks(&self) -> u32 {
        match self {
            SimRate::RealTime => 1,
            SimRate::ThreeSecondsPerSecond => 3,
            SimRate::TenSecondsPerSecond => 10,
            SimRate::MinutePerSecond => 60,
            SimRate::FiveMinsPerSecond => 5 * 60,
            SimRate::FifteenMinsPerSecond => 15 * 60,
            SimRate::ThirtyMinsPerSecond => 30 * 60,
            SimRate::HourPerSecond => 3600,
            SimRate::ThreeHourPerSecond => 3 * 3600,
            SimRate::SixHourPerSecond => 6 * 3600,
            SimRate::TwelveHourPerSecond => 12 * 3600,
            SimRate::DayPerSecond => 3600 * 24,
            SimRate::WeekPerSecond => 3600 * 24 * 7,
            SimRate::MonthPerSecond => 3600 * 24 * 30,
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        all::<Self>()
    }
}
