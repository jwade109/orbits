use crate::*;

#[derive(Resource, Serialize, Deserialize)]
pub struct Settings {
    pub draw_spatial_lut: bool,
    pub draw_spacecraft_grids: bool,
    pub draw_terrain_rgb: bool,
    pub show_wireframes: bool,
    pub draw_inventories: bool,
    pub draw_blueprints: bool,
    pub draw_docking_info: bool,
    pub draw_thruster_states: bool,
    pub dig_with_mouse: bool,
    pub rotation_locked: bool,
    pub infinite_fuel: bool,
    pub show_terrain_info: bool,
    pub show_cursor_info: bool,
    pub show_time_controls: bool,
    pub draw_camera_debug: bool,
}

impl Settings {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(path)?;
        let settings: Self = serde_yaml::from_str(&s)?;
        Ok(settings)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            draw_spatial_lut: false,
            draw_spacecraft_grids: false,
            draw_terrain_rgb: false,
            show_wireframes: false,
            draw_inventories: false,
            draw_blueprints: false,
            draw_docking_info: false,
            draw_thruster_states: false,
            dig_with_mouse: false,
            rotation_locked: false,
            infinite_fuel: false,
            show_terrain_info: false,
            show_cursor_info: false,
            show_time_controls: false,
            draw_camera_debug: false,
        }
    }
}

pub fn save_settings_on_change(ctx: Res<ProgramContext>, settings: Res<Settings>) {
    if !settings.is_changed() {
        return;
    }

    let filepath = ctx.settings_path();

    match serde_yaml::to_string(&*settings) {
        Ok(s) => {
            std::fs::write(filepath, s);
        }
        Err(e) => {
            error!("Failed to write settings: {:?}", e);
        }
    }
}
