use crate::game_version_two::*;

#[derive(Resource, Serialize, Deserialize)]
pub struct Settings {
    pub draw_spatial_lut: bool,
    pub draw_spacecraft_grids: bool,
    pub draw_terrain_rgb: bool,
    pub show_wireframes: bool,
    pub draw_debug_inventories: bool,
    pub draw_docking_port_info: bool,
    pub draw_thruster_states: bool,
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
            draw_debug_inventories: false,
            draw_docking_port_info: false,
            draw_thruster_states: false,
        }
    }
}
