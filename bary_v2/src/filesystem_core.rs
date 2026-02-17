use std::path::{Path, PathBuf};

use bary_core::prelude::PartDatabase;
use bary_v1::args::ProgramContext;
use bevy::prelude::*;

use crate::{PartsResource, Settings, load_parts_from_dir_2};

#[derive(Resource, Deref)]
pub struct ShipNames(pub Vec<String>);

pub fn load_names_from_file(filename: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

pub struct FilesystemPlugin {
    pub path: Option<PathBuf>,
}

impl Plugin for FilesystemPlugin {
    fn build(&self, app: &mut App) {
        let ctx = ProgramContext::new(self.path.clone());
        let settings = Settings::from_file(&ctx.settings_path()).unwrap_or(Settings::default());
        let parts = load_parts_from_dir_2(&ctx).unwrap_or(PartDatabase::default());
        let ship_names = load_names_from_file(&ctx.names_path()).unwrap_or(vec![]);

        dbg!(&ctx);

        app.insert_resource(PartsResource(parts));
        app.insert_resource(settings);
        app.insert_resource(ctx);
        app.insert_resource(ShipNames(ship_names));
    }
}
