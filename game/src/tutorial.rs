use crate::camera_controller::camera_span_meters;
use crate::game::GameState;
use crate::sim_rate::SimRate;
use serde::{Deserialize, Serialize};
use starling::prelude::Nanotime;
use starling::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum GoalCondition {
    Rendezvous {
        ownship: EntityId,
        target: EntityId,
    },
    SetTarget {
        ownship: EntityId,
        target: EntityId,
    },
    SelectVehicle(EntityId),
    ThrustForward(EntityId),
    ThrustBackward(EntityId),
    HoldAttitude(EntityId, u16),
    FocusCamera(EntityId),
    ZoomCamera,
    Apoapsis(EntityId, f64, f64),
    Periapsis(EntityId, f64, f64),
    Orbit {
        vehicle_id: EntityId,
        planet_id: EntityId,
        rp: f64,
        ra: f64,
        argp: f64,
        tol: f64,
    },
    Prograde(EntityId),
    Retrograde(EntityId),
    Idle(EntityId),
    TimeWarp,
    CancelTimeWarp,
    Pause,
}

#[derive(Debug, Clone, Copy)]
pub struct GoalDuration {
    pub required: Nanotime,
    pub actual: Nanotime,
}

#[derive(Debug, Clone, Copy)]
pub struct Goal {
    pub is_complete: bool,
    pub is_permanent: bool,
    pub cond: GoalCondition,
    pub dur: Option<GoalDuration>,
}

impl Goal {
    pub fn new(cond: GoalCondition) -> Self {
        Self {
            is_complete: false,
            is_permanent: false,
            cond,
            dur: None,
        }
    }

    pub fn with_duration(mut self, duration: Nanotime) -> Self {
        self.dur = Some(GoalDuration {
            required: duration,
            actual: Nanotime::ZERO,
        });
        self
    }

    pub fn as_impermanent(mut self) -> Self {
        self.is_permanent = false;
        self
    }

    pub fn update(&mut self, state: &GameState) {
        if self.is_complete && self.is_permanent {
            return;
        }

        let satisfied = is_satisfied(state, &self.cond).unwrap_or(false);

        if let Some(gd) = &mut self.dur {
            if satisfied {
                if !state.paused {
                    gd.actual += PHYSICS_CONSTANT_DELTA_TIME;
                    self.is_complete = gd.actual >= gd.required;
                }
            } else {
                gd.actual = Nanotime::ZERO;
                self.is_complete = false;
            }
        } else {
            self.is_complete = satisfied;
        }
    }

    pub fn progress(&self) -> f32 {
        if let Some(gd) = &self.dur {
            gd.actual.to_secs() / gd.required.to_secs()
        } else {
            self.is_complete as u8 as f32
        }
    }
}

fn is_satisfied(state: &GameState, cond: &GoalCondition) -> Option<bool> {
    match cond {
        GoalCondition::Rendezvous { ownship, target } => {
            let pva = state.universe.pv(*ownship)?;
            let pvb = state.universe.pv(*target)?;
            let ds = pva.pos.distance(pva.pos);
            let dv = pva.vel.distance(pvb.vel);
            Some(ds < 100.0 && dv < 3.0)
        }
        GoalCondition::SetTarget { ownship, target } => {
            let sv = state.universe.spacecraft.get(ownship)?;
            Some(sv.target() == Some(*target))
        }
        GoalCondition::SelectVehicle(v) => Some(state.orbital_context.piloting == Some(*v)),
        GoalCondition::ThrustForward(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            Some(sv.vehicle.is_thrusting() && sv.vehicle.body_frame_accel().linear.x > 2.0)
        }
        GoalCondition::ThrustBackward(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            Some(sv.vehicle.is_thrusting() && sv.vehicle.body_frame_accel().linear.x < -2.0)
        }
        GoalCondition::HoldAttitude(id, heading) => {
            if Some(*id) != state.piloting() {
                return Some(false);
            }
            let sv = state.universe.spacecraft.get(id)?;
            let heading = *heading as f64 / 180.0 * PI_64;
            let error_deg = wrap_pi_npi_f64(sv.body.angle - heading).abs().to_degrees();
            let rate = sv.body.angular_velocity.to_degrees();
            Some(error_deg.abs() < 5.0 && rate.abs() < 3.0)
        }
        GoalCondition::FocusCamera(id) => Some(state.orbital_context.camera.is_following(*id)),
        GoalCondition::ZoomCamera => {
            let min = 5.0;
            let max = 250.0;
            let meters = camera_span_meters(
                state.input.screen_bounds.span,
                &state.orbital_context.camera,
            );

            Some(min <= meters.length() && meters.length() <= max)
        }
        GoalCondition::Periapsis(id, min, max) => {
            let sv = state.universe.spacecraft.get(id)?;
            let orbit = sv.current_orbit()?;
            let r = orbit.1.periapsis_r();
            Some(*min <= r && r <= *max)
        }
        GoalCondition::Apoapsis(id, min, max) => {
            let sv = state.universe.spacecraft.get(id)?;
            let orbit = sv.current_orbit()?;
            let r = orbit.1.apoapsis_r();
            Some(*min <= r && r <= *max)
        }
        GoalCondition::Orbit {
            vehicle_id,
            planet_id,
            rp,
            ra,
            argp,
            tol,
        } => {
            let sv = state.universe.spacecraft.get(vehicle_id)?;
            if sv.parent() != *planet_id {
                return Some(false);
            }
            let orbit = sv.current_orbit()?;
            let target_orbit =
                SparseOrbit::new(*ra, *rp, *argp, orbit.1.body, Nanotime::ZERO, false)?;
            let ta = target_orbit.apoapsis();
            let tp = target_orbit.periapsis();
            let a = orbit.1.apoapsis();
            let p = orbit.1.periapsis();
            Some(a.distance(ta) < *tol && p.distance(tp) < *tol)
        }
        GoalCondition::Prograde(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            Some(
                sv.controller.mode().is_prograde()
                    && sv.controller.status() != VehicleControlStatus::ComingAbout,
            )
        }
        GoalCondition::Retrograde(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            Some(
                sv.controller.mode().is_retrograde()
                    && sv.controller.status() != VehicleControlStatus::ComingAbout,
            )
        }
        GoalCondition::Idle(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            Some(sv.controller.mode().is_idle())
        }
        GoalCondition::TimeWarp => Some(
            state.universe_ticks_per_game_tick.as_ticks() >= SimRate::FiveMinsPerSecond.as_ticks(),
        ),
        GoalCondition::CancelTimeWarp => {
            Some(state.universe_ticks_per_game_tick == SimRate::RealTime)
        }
        GoalCondition::Pause => Some(state.paused),
    }
}

pub fn get_goal_text(cond: &GoalCondition, state: &GameState) -> Option<String> {
    match cond {
        GoalCondition::SelectVehicle(id) => {
            let sv = state.universe.spacecraft.get(&id)?;
            let title = sv.vehicle.title_with_id(*id);
            Some(format!(
                "Select vessel \"{}\" by left-clicking on it.",
                title
            ))
        }
        GoalCondition::SetTarget { ownship, target } => {
            let ov = state.universe.spacecraft.get(ownship)?;
            let tv = state.universe.spacecraft.get(target)?;
            let ot = ov.vehicle.name();
            let tt = tv.vehicle.title_with_id(*target);
            Some(format!(
                "Set the target of \"{}\" to \"{}\" by right-clicking on it.",
                ot, tt,
            ))
        }
        GoalCondition::Rendezvous { ownship, target } => {
            let ov = state.universe.spacecraft.get(ownship)?;
            let tv = state.universe.spacecraft.get(target)?;
            let ot = ov.vehicle.name();
            let tt = tv.vehicle.name();
            Some(format!("Rendezvous \"{}\" with \"{}\".", ot, tt,))
        }
        GoalCondition::ThrustForward(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Apply forwards thrust to \"{}\".", n))
        }
        GoalCondition::ThrustBackward(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Apply backwards thrust to \"{}\".", n))
        }
        GoalCondition::HoldAttitude(id, deg) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Hold heading of \"{}\" at {} degrees.", n, deg))
        }
        GoalCondition::FocusCamera(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!(
                "Set the camera to follow \"{}\" with ctrl+left-click.",
                n
            ))
        }
        GoalCondition::ZoomCamera => Some(format!("Press V to zoom in on the selected vehicle.",)),
        GoalCondition::Periapsis(_, min, max) => Some(format!(
            "Set periapsis to between {} and {}.",
            distance_str(*min),
            distance_str(*max)
        )),
        GoalCondition::Apoapsis(_, min, max) => Some(format!(
            "Set apoapsis to between {} and {}.",
            distance_str(*min),
            distance_str(*max)
        )),
        GoalCondition::Orbit {
            vehicle_id,
            planet_id,
            ..
        } => {
            let sv = state.universe.spacecraft.get(vehicle_id)?;
            let n = sv.vehicle.name_with_id(*vehicle_id);
            let planet = state.universe.get_planet(*planet_id)?;
            Some(format!(
                "Place \"{}\" into the specified orbit around {}.",
                n, planet.name
            ))
        }
        GoalCondition::Prograde(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to PROGRADE.", n))
        }
        GoalCondition::Retrograde(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to RETROGRADE.", n))
        }
        GoalCondition::Idle(id) => {
            let sv = state.universe.spacecraft.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to IDLE.", n))
        }
        GoalCondition::TimeWarp => Some(format!("Accelerate time using time warp by pressing [.]")),
        GoalCondition::CancelTimeWarp => {
            Some(format!("Cancel time warp with [/] to return to real time."))
        }
        GoalCondition::Pause => Some(format!("Pause the game with [SPACE].")),
    }
}

#[derive(Debug, Clone)]
pub struct Tutorial {
    pub chapters: Vec<TutorialChapter>,
    pub current: usize,
}

impl Tutorial {
    fn from_storage(chapters: Vec<TutorialChapterFileStorage>) -> Self {
        Self {
            chapters: chapters
                .into_iter()
                .map(|t| TutorialChapter::from_storage(t))
                .collect(),
            current: 0,
        }
    }

    pub fn current(&self) -> Option<&TutorialChapter> {
        self.chapters.get(self.current)
    }

    pub fn current_is_last(&self) -> bool {
        self.current + 1 == self.chapters.len()
    }

    pub fn update(&mut self, state: &GameState) -> bool {
        let mut any_chapter_completed = false;
        if let Some(chapter) = self.chapters.get_mut(self.current) {
            let before = chapter.is_complete;
            for cond in &mut chapter.conditions {
                cond.update(state);
            }
            chapter.is_complete = chapter.is_complete();
            let after = chapter.is_complete;
            any_chapter_completed |= !before && after;
        }
        any_chapter_completed
    }

    pub fn is_complete(&self) -> bool {
        self.chapters.iter().all(|c| c.is_complete())
    }

    pub fn next(&mut self, force: bool) {
        if let Some(c) = self.current() {
            if (force || c.is_complete()) && self.current < self.chapters.len() {
                self.current += 1;
            }
        }
    }

    pub fn prev(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TutorialChapter {
    pub title: String,
    pub intro: String,
    pub conditions: Vec<Goal>,
    pub ending: String,
    pub is_complete: bool,
}

impl TutorialChapter {
    fn from_storage(chapter: TutorialChapterFileStorage) -> Self {
        Self {
            title: chapter.title,
            intro: chapter.intro,
            conditions: chapter
                .conditions
                .iter()
                .map(|s| {
                    let mut g = Goal::new(s.cond);
                    g.is_permanent = s.is_permanent;
                    if s.seconds > 0 {
                        let t = Nanotime::secs(s.seconds.into());
                        g.dur = Some(GoalDuration {
                            required: t,
                            actual: Nanotime::ZERO,
                        });
                    }
                    g
                })
                .collect(),
            ending: chapter.ending,
            is_complete: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.is_complete | self.conditions.iter().all(|g| g.is_complete)
    }
}

impl TutorialChapter {
    pub fn new(
        title: impl Into<String>,
        intro: impl Into<String>,
        conditions: &[(GoalCondition, bool)],
        ending: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            intro: intro.into(),
            conditions: conditions
                .iter()
                .map(|(c, is_permanent)| {
                    let mut g = Goal::new(*c);
                    g.is_permanent = *is_permanent;
                    g
                })
                .collect(),
            ending: ending.into(),
            is_complete: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TutorialChapterFileStorage {
    title: String,
    intro: String,
    conditions: Vec<ConditionFileStorage>,
    ending: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConditionFileStorage {
    cond: GoalCondition,
    is_permanent: bool,
    seconds: u16,
}

pub fn load_tutorial_from_file(
    path: &std::path::Path,
) -> Result<Tutorial, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(path)?;
    let storage = serde_yaml::from_str::<Vec<TutorialChapterFileStorage>>(&s)?;
    let tutorial = Tutorial::from_storage(storage);
    Ok(tutorial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_file() {
        let t = load_tutorial_from_file(std::path::Path::new("../assets/tutorial.yaml")).unwrap();
        println!("{:#?}", t);
    }
}
