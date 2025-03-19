#![allow(private_interfaces)]
#![allow(dead_code, unused_imports)]

use crate::planetary::GameState;
use bevy::prelude::*;
use starling::prelude::*;

pub struct NotificationsPlugin;

impl Plugin for NotificationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_warnings);
    }
}

#[derive(Component)]
struct NotifComponent(usize);

const WARNING_Z_INDEX: f32 = 9.0;

pub fn update_warnings(
    state: Res<GameState>,
    mut query: Query<(Entity, &NotifComponent, &mut Transform, &mut Text2d)>,
    mut commands: Commands,
) {

    // for (e, n, _, _) in &query {
    //     if n.0 >= state.notifications.len() {
    //         commands.entity(e).despawn();
    //     }
    // }

    // let position = |n: &Notification| -> Option<Vec2> {
    //     state
    //         .scenario
    //         .lup(n.parent, state.sim_time)
    //         .map(|lup| lup.pv().pos + n.offset + n.jitter + Vec2::Y * 50.0)
    // };

    // for (i, notif) in state.notifications.iter().enumerate() {
    //     let pos = match position(notif) {
    //         Some(p) => p,
    //         None => continue,
    //     };

    //     if let Some((_, _, mut tf, mut text)) = query.iter_mut().find(|(_, n, _, _)| n.0 == i) {
    //         tf.translation = pos.extend(tf.translation.z);
    //         text.0 = notif.message.clone();
    //     } else {
    //         commands.spawn((
    //             NotifComponent(i),
    //             Transform::from_translation(pos.extend(WARNING_Z_INDEX)),
    //             Text2d(notif.message.clone()),
    //         ));
    //     }
    // }
}
