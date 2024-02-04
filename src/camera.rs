use anyhow::Result;
use bevy::prelude::*;

use crate::{
    handle_errors,
    player::{Player, PlayerPhysics},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update
                .pipe(handle_errors)
                .run_if(resource_exists::<Player>()),
        );
    }
}

fn update(
    time: Res<Time>,
    player: Query<&Transform, (With<PlayerPhysics>, Without<Camera3d>)>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) -> Result<()> {
    let mut camera = camera.get_single_mut()?;
    let player = player.get_single()?;

    let mut newpos = player.translation.clone();
    newpos.x -= 1.0;
    newpos.y += 2.0;
    newpos.z += 8.0;

    let speed = 20.0 * time.delta_seconds();
    let new_transform = Transform::from_translation(newpos).looking_at(player.translation, Vec3::Y);

    camera.translation = camera.translation.lerp(new_transform.translation, speed);
    camera.rotation = new_transform.rotation;

    Ok(())
}
