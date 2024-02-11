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
    player: Res<Player>,
    transform: Query<&Transform, (With<PlayerPhysics>, Without<Camera3d>)>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) -> Result<()> {
    let mut speed = 10.0 * time.delta_seconds();

    let transform = transform.get_single()?;
    let lookat = transform.translation.clone();
    let newpos = Vec3::from((
        transform.translation.x - 1.0,
        transform.translation.y + 2.0,
        transform.translation.z + 8.0,
    ));

    let mut new_transform = Transform::from_translation(newpos).looking_at(lookat, Vec3::Y);
    if player.is_space {
        new_transform.translation.z -= 4.0;
        new_transform.translation.y -= 1.0;
        speed *= 0.5;
    }

    if let Some(ref view) = player.view_controller {
        new_transform = Transform::from_translation(view.from).looking_at(view.to, Vec3::Y);
    }

    for mut camera in cameras.iter_mut() {
        camera.translation = camera.translation.lerp(new_transform.translation, speed);
        camera.rotation = camera.rotation.slerp(new_transform.rotation, speed);
    }

    Ok(())
}
