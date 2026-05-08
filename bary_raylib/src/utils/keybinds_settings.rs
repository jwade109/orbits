use crate::utils::{InputAction, KeyMapping};
use bary_core::prelude::BaryResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeybindContext {
    Piloting,
    Editor,
    Free,
    Camera,
    TimeControl,
    Console,
    MainMenu,
    DebugMenu,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalKeybinds {
    keybinds: BTreeMap<KeybindContext, KeyMapping>,
}

impl GlobalKeybinds {
    pub fn iter(&self) -> impl Iterator<Item = (&KeybindContext, &KeyMapping)> {
        self.keybinds.iter()
    }

    pub fn add_mapping(&mut self, ctx: KeybindContext, key: rdev::Key, action: InputAction) {
        self.keybinds
            .entry(ctx)
            .and_modify(|e| {
                e.set_mapping(key, action);
            })
            .or_insert(KeyMapping::new([(key, action)]));
    }
}

pub fn load_keybinds_from_file(path: &str) -> BaryResult<GlobalKeybinds> {
    let contents = std::fs::read_to_string(path)?;
    let kb: GlobalKeybinds = serde_yaml::from_str(&contents)?;
    Ok(kb)
}

#[cfg(test)]
mod tests {
    use crate::utils::InputAction;

    use super::*;

    #[test]
    fn serialize_global_keybinds() {
        let mut kb = GlobalKeybinds::default();

        kb.add_mapping(
            KeybindContext::Editor,
            rdev::Key::KeyD,
            InputAction::EditorDelete,
        );

        kb.add_mapping(
            KeybindContext::Editor,
            rdev::Key::KeyQ,
            InputAction::EditorContext,
        );

        kb.add_mapping(
            KeybindContext::DebugMenu,
            rdev::Key::KeyR,
            InputAction::DebugRecord,
        );

        let s = serde_yaml::to_string(&kb).unwrap();
        println!("{}", s);

        let contents = std::fs::read_to_string("../assets/keybinds.yaml").unwrap();

        let kb: GlobalKeybinds = serde_yaml::from_str(&contents).unwrap();

        dbg!(kb);
    }
}
