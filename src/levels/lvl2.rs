use anyhow::Result;
use bevy::prelude::*;

use crate::{
    components::{
        code::Code, fan::Fan, loading::Loading, security_camera::SecurityCamera, socket::Socket,
        switch::Switch,
    },
    game_scene::{GameScene, GameSceneData},
    handle_errors,
    player::Player,
    utils::reduce_to_root,
    GameState, Restart,
};

use super::{GameLevel, LoadLevel};

struct Entities {
    socket_end: Entity,
    cam1: Entity,
    switch1: Entity,
    code1: Entity,
    fan1: Entity,
}

#[derive(Resource)]
pub struct Level2 {
    scene_data: GameSceneData,
    entities: Option<Entities>,
}

impl GameScene for Level2 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            entities: None,
        }
    }
}

impl GameLevel for Level2 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            ((
                ready.run_if(resource_added::<Level2>()),
                (process_sensors, process_animations.pipe(handle_errors))
                    .before(ready)
                    .run_if(resource_exists::<Player>())
                    .run_if(resource_exists::<Level2>())
                    .run_if(not(any_with_component::<Loading>())),
            )
                .run_if(in_state(state.clone())),),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level2>("lvl1.glb", 2));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level2>();
}

fn ready(
    mut commands: Commands,
    mut level: ResMut<Level2>,
    entities: Query<(Entity, &Name)>,
    children: Query<&Parent>,
) {
    let mut socket_end = None;
    let mut cam1 = None;
    let mut switch1 = None;
    let mut code1 = None;
    let mut fan1 = None;

    let anims = &level.scene_data.animations;

    let root = level.scene_data.root;
    for (entity, name) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        let mut entity = commands.entity(entity);
        match name.as_str() {
            "socket_start.002" => {
                entity.insert((Loading, Socket::new(true)));
            }
            "socket_end.002" => {
                socket_end = Some(entity.insert((Loading, Socket::new(false))).id())
            }
            "camera.002" => cam1 = Some(entity.insert((Loading, SecurityCamera::new())).id()),
            "switch.003" => {
                switch1 = Some(entity.insert((Loading, Switch::new(anims))).id());
            }
            "code.002" => code1 = Some(entity.insert((Loading, Code::new(1824))).id()),
            "fan.002" => fan1 = Some(entity.insert((Loading, Fan::new())).id()),
            _ => {}
        };
    }

    level.entities = Some(Entities {
        socket_end: socket_end.unwrap(),
        cam1: cam1.unwrap(),
        switch1: switch1.unwrap(),
        code1: code1.unwrap(),
        fan1: fan1.unwrap(),
    });
}

fn process_sensors(
    mut commands: Commands,
    mut game_state: ResMut<NextState<GameState>>,
    level: Res<Level2>,
    sockets: Query<&Socket>,
    mut sec_cams: Query<&mut SecurityCamera>,
    switches: Query<&Switch>,
    codes: Query<&Code>,
    mut fans: Query<&mut Fan>,
) {
    let Some(entities) = &level.entities else {
        return;
    };

    let socket_end = sockets.get(entities.socket_end).unwrap();
    let mut cam1 = sec_cams.get_mut(entities.cam1).unwrap();
    let switch1 = switches.get(entities.switch1).unwrap();
    let code1 = codes.get(entities.code1).unwrap();
    let mut fan1 = fans.get_mut(entities.fan1).unwrap();

    if code1.activated() {
        fan1.spinning = false;
    }

    if switch1.activated() {
        cam1.active = false;
    }

    if cam1.triggered {
        commands.insert_resource(Restart(GameState::Level2));
        game_state.set(GameState::Restart);
    }

    if socket_end.connected() {
        game_state.set(GameState::Level3);
    }
}

fn process_animations(level: Res<Level2>) -> Result<()> {
    Ok(())
}
