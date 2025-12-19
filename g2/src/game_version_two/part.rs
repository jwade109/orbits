use crate::game_version_two::*;

#[derive(Component, Deserialize, Serialize, Debug, Clone)]
pub struct InventoryData {
    slots: usize,
}

pub fn load_parts_from_dir(ctx: &ProgramContext) {
    info!(?ctx);

    let parts_dir = ctx.parts_dir();

    for file in ok_or_return!(std::fs::read_dir(&parts_dir)) {
        let file = ok_or_continue!(file);
        let part_name = ok_or_continue!(file.file_name().into_string());

        let data_path = file.path().join("metadata.yaml");

        let data = match PartPrototype::from_file(&data_path) {
            Ok(data) => data,
            Err(e) => {
                error!("\n   Failed to load part \"{}\": {:?}\n", part_name, e);
                continue;
            }
        };

        info!("{} -> {:?}", part_name, data);
    }
}
