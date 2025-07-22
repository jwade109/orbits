use enum_iterator::Sequence;

#[derive(Debug, Clone, Copy, Sequence, PartialEq, Eq)]
pub enum SimRate {
    Paused,
    RealTime,
    HourPerSecond,
    DayPerSecond,
    WeekPerSecond,
    MonthPerSecond
}

impl SimRate {
    pub fn is_paused(&self) -> bool {
        match self {
            Self::Paused => true,
            _ => false,
        }
    }

    pub fn slower(&mut self) {
        *self = enum_iterator::previous(self).unwrap_or(Self::Paused);
    }

    pub fn faster(&mut self) {
        *self = enum_iterator::next(self).unwrap_or(Self::Paused);
    }

    pub fn as_f32(&self) -> f32 {
        let r = match self {
            Self::Paused => 1,
            Self::RealTime => 1,
            Self::HourPerSecond => 3600,
            Self::DayPerSecond => 86400,
            _ => todo!(),
        };

        r as f32
    }
}
