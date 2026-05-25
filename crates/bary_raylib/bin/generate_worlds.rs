use bary_core::prelude::*;
use bary_factory::*;
use bary_raylib::{persistence::save_world, world_builder::WorldBuilder};
use bary_sim::*;

fn main() -> BaryResult<()> {
    let world = WorldBuilder::new()
        .assets()
        .blueprint(("pollux", 0))
        .blueprint(("pollux", 2))
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .blueprint("foundation")
        .blueprint("miner")
        .blueprint("icecream")
        .spawn(("pollux", 0), "", (30.0, 0.0, 0.0))
        .spawn(("pollux", 2), "", (0.0, 0.0, 0.0))
        .insert_source((19, 7), Item::Magnesium)
        .insert_source((20, 7), Item::Iron)
        .insert_source((21, 7), Item::Titanium)
        .set_recipe((21, 7), RecipeListing::TitaniumLattice)
        .insert_source((22, 11), Item::Water)
        .insert_pipe((22, 11), (22, 10))
        .set_recipe((27, 7), RecipeListing::WaterElectrolysis)
        .insert_pipe((17, 10), (15, 10))
        .insert_pipe((27, 7), (34, 7))
        .insert_pipe((26, 7), (25, 6))
        .spawn("remora", "", (10.0, 30.0, 0.1))
        .spawn("miner", "", (-9.0, 12.0, -0.3))
        .spawn("remora", "", (-7.0, 23.0, 0.7))
        .spawn("bellerophon", "", (130.0, 50.0, 0.1))
        .command(WorldDelta::SetSpeed(10))
        .command(WorldDelta::Ping(Vec2::ZERO))
        .command(WorldDelta::Ping(Vec2::splat(10.0)))
        .asteroid((-80.0, 30.0, 0.1), 20.0, 391)
        .asteroid((60.0, 300.0, 0.7), 50.0, 2384)
        // .asteroid((400.0, -2000.0, 0.7), 500.0, 9312)
        .build();

    save_world("saves/scenario_a", &world, true)
}
