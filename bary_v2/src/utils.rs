use bevy::prelude::*;

pub fn swallow_optional(In(_): In<Option<()>>) {}

pub fn swallow_result<T: std::fmt::Debug>(In(r): In<Result<(), T>>) {
    if let Err(e) = r {
        error!(?e);
    }
}
