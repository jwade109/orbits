use crate::game_version_two::*;

#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct PartDatabase(HashMap<String, PartPrototype>);

pub fn load_parts_from_dir(
    ctx: &ProgramContext,
) -> Result<PartDatabase, Box<dyn std::error::Error>> {
    info!(?ctx);

    let parts_dir = ctx.parts_dir();

    let mut db = PartDatabase::default();

    for file in std::fs::read_dir(&parts_dir)? {
        let file = file?;
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

        db.insert(part_name, data);
    }

    Ok(db)
}
