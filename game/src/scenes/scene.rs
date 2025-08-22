use enum_iterator::Sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum SceneType {
    Orbital,
    Telescope,
    Editor,
    MainMenu,
}

impl SceneType {
    pub fn all() -> impl Iterator<Item = SceneType> {
        enum_iterator::all::<SceneType>()
    }
}
