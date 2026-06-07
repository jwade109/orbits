use bary_core::prelude::*;
use bary_input::InputState;
use bary_ui::Tree;
use log::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMessage {
    Exit,
    SaveFile,
    OpenEditor,
    AltMode,
    DebugText,
    SimSpeed(u32),
    DockingShift(Rotation),
    DockingRotate,
    DockingActivate,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiInteractionState {
    is_on_gui: bool,
    screen_pos: Option<Vec2>,
    hot: Option<ClickInfo>,
    active: Option<ClickInfo>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClickInfo {
    pub msg: UiMessage,
    pub pos: Vec2,
}

impl UiInteractionState {
    pub fn is_on_gui(&self) -> bool {
        self.is_on_gui
    }

    pub fn active(&self) -> Option<UiMessage> {
        self.active.map(|e| e.msg)
    }

    pub fn update(
        &mut self,
        ui: &Tree<UiMessage>,
        screen_pos: Option<Vec2>,
        input: &InputState,
    ) -> Option<ClickInfo> {
        self.is_on_gui = false;
        self.hot = None;
        let last_pos = self.screen_pos;
        self.screen_pos = screen_pos;

        if let Some(p) = screen_pos {
            for node in ui.iter() {
                let aabb = node.aabb();
                let contains = aabb.contains(p);

                self.is_on_gui |= contains;

                let Some(onclick) = node.on_click() else {
                    continue;
                };

                let relative = p - aabb.lower();

                if contains {
                    self.hot = Some(ClickInfo {
                        msg: *onclick,
                        pos: relative,
                    });

                    if input.just_pressed(rdev::Button::Left) {
                        self.active = self.hot;
                    }
                }
            }
        }

        let mut ret = None;

        if input.is_key_pressed(rdev::Button::Left) {
            if let Some((last, (cur, active))) = last_pos.zip(self.screen_pos.zip(self.active)) {
                if last != cur {
                    info!("Drag! {} -> {} ({:?})", last, cur, active);
                }
            }
        }

        if input.just_released(rdev::Button::Left) {
            if self.active.map(|e| e.msg) == self.hot.map(|e| e.msg) {
                ret = self.active;
            }
            self.active = None;
        }

        ret
    }
}
