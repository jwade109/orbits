#[cfg(test)]
mod tests {
    use bary_v2::*;
    use bevy::prelude::*;
    use std::path::PathBuf;

    fn get_test_app() -> App {
        let mut app = App::new();

        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");

        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ImagePlugin::default(),
            FilesystemPlugin { path: Some(p) },
            world_tick_driver_plugin,
            spacecraft_simtick_plugin,
            grid_lookup_plugin,
        ));

        app.insert_resource(ManualControl::default());

        app
    }

    #[test]
    fn application_test() {
        let mut app = get_test_app();

        // TODO move to plugin
        app.add_observer(handle_sc_events);

        app.world_mut().run_schedule(FixedUpdate);

        let ticks = app.world().get_resource::<TickStatistics>().unwrap();

        dbg!(ticks);

        assert_eq!(ticks.ticks, 10);

        app.world_mut()
            .commands()
            .trigger(SpacecraftEvent::SpawnPart {
                name: "cargo".into(),
                pos: Vec2::ZERO,
                angle: 0.0,
            });

        app.world_mut().flush();

        // app.world_mut().run_schedule(FixedUpdate);

        let mut grid = app.world_mut().query::<&SpacecraftGrid>();

        assert_eq!(grid.query(app.world()).iter().len(), 1);

        let grid = grid.single(app.world()).unwrap();

        dbg!(&grid);
    }
}
