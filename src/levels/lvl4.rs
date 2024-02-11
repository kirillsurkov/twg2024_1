use anyhow::Result;
use bevy::{prelude::*, render::view::NoFrustumCulling};

use crate::{
    components::{
        code::Code, fan::Fan, gate::Gate, loading::Loading, security_camera::SecurityCamera,
        socket::Socket, switch::Switch,
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
    gate1: Entity,
    gate2: Entity,
    gate3: Entity,
    gate4: Entity,
    gate5: Entity,
    switch1: Entity,
    switch2: Entity,
    switch3: Entity,
    switch4: Entity,
    switch5: Entity,
    code1: Entity,
    code2: Entity,
    code3: Entity,
    code4: Entity,
    fan1: Entity,
    fan2: Entity,
    fan3: Entity,
    cam1: Entity,
}

#[derive(Resource)]
pub struct Level4 {
    scene_data: GameSceneData,
    entities: Option<Entities>,
}

impl GameScene for Level4 {
    fn from_scene_data(data: GameSceneData) -> Self {
        Self {
            scene_data: data,
            entities: None,
        }
    }
}

impl GameLevel for Level4 {
    fn build(state: GameState, app: &mut App) {
        app.add_systems(OnEnter(state.clone()), setup);
        app.add_systems(OnExit(state.clone()), cleanup);
        app.add_systems(
            Update,
            ((
                ready.run_if(resource_added::<Level4>()),
                (process_sensors, process_animations.pipe(handle_errors))
                    .before(ready)
                    .run_if(resource_exists::<Player>())
                    .run_if(resource_exists::<Level4>())
                    .run_if(not(any_with_component::<Loading>())),
            )
                .run_if(in_state(state.clone())),),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(LoadLevel::new::<Level4>("lvl1.glb", 4));
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<Level4>();
}

fn ready(
    mut commands: Commands,
    mut level: ResMut<Level4>,
    entities: Query<(Entity, &Name)>,
    children: Query<&Parent>,
) {
    let mut socket_end = None;
    let mut gate1 = None;
    let mut gate2 = None;
    let mut gate3 = None;
    let mut gate4 = None;
    let mut gate5 = None;
    let mut switch1 = None;
    let mut switch2 = None;
    let mut switch3 = None;
    let mut switch4 = None;
    let mut switch5 = None;
    let mut code1 = None;
    let mut code2 = None;
    let mut code3 = None;
    let mut code4 = None;
    let mut fan1 = None;
    let mut fan2 = None;
    let mut fan3 = None;
    let mut cam1 = None;

    let anims = &level.scene_data.animations;

    let root = level.scene_data.root;
    for (entity, name) in entities.iter() {
        if !reduce_to_root(&children, entity, false, |f, r| f || (r == root)) {
            continue;
        }
        let mut entity = commands.entity(entity);
        match name.as_str() {
            "socket_start.004" => {
                entity.insert((Loading, Socket::new(true)));
            }
            "socket_end.004" => {
                socket_end = Some(entity.insert((Loading, Socket::new(false))).id())
            }
            "gate.003" => gate1 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "gate.004" => gate2 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "gate.005" => gate3 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "gate.006" => gate4 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "gate.007" => gate5 = Some(entity.insert((Loading, Gate::new(anims))).id()),
            "switch.004" => switch1 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "switch.007" => switch2 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "switch.008" => switch3 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "switch.009" => switch4 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "switch.010" => switch5 = Some(entity.insert((Loading, Switch::new(anims))).id()),
            "code.003" => code1 = Some(entity.insert((Loading, Code::new(9835))).id()),
            "code.006" => code2 = Some(entity.insert((Loading, Code::new(0152))).id()),
            "code.007" => code3 = Some(entity.insert((Loading, Code::new(5489))).id()),
            "code.008" => code4 = Some(entity.insert((Loading, Code::new(9845))).id()),
            "fan.010" => fan1 = Some(entity.insert((Loading, Fan::new())).id()),
            "fan.012" => fan2 = Some(entity.insert((Loading, Fan::new())).id()),
            "fan.014" => fan3 = Some(entity.insert((Loading, Fan::new())).id()),
            "camera.003" => cam1 = Some(entity.insert((Loading, SecurityCamera::new())).id()),
            _ => {}
        };
    }

    level.entities = Some(Entities {
        socket_end: socket_end.unwrap(),
        gate1: gate1.unwrap(),
        gate2: gate2.unwrap(),
        gate3: gate3.unwrap(),
        gate4: gate4.unwrap(),
        gate5: gate5.unwrap(),
        switch1: switch1.unwrap(),
        switch2: switch2.unwrap(),
        switch3: switch3.unwrap(),
        switch4: switch4.unwrap(),
        switch5: switch5.unwrap(),
        code1: code1.unwrap(),
        code2: code2.unwrap(),
        code3: code3.unwrap(),
        code4: code4.unwrap(),
        fan1: fan1.unwrap(),
        fan2: fan2.unwrap(),
        fan3: fan3.unwrap(),
        cam1: cam1.unwrap(),
    });
}

fn process_sensors(
    mut commands: Commands,
    mut game_state: ResMut<NextState<GameState>>,
    level: Res<Level4>,
    sockets: Query<&Socket>,
    mut gates: Query<&mut Gate>,
    switches: Query<&Switch>,
    codes: Query<&Code>,
    mut fans: Query<&mut Fan>,
    mut sec_cams: Query<&mut SecurityCamera>,
) {
    let Some(entities) = &level.entities else {
        return;
    };

    let socket_end = sockets.get(entities.socket_end).unwrap();
    let [mut gate1, mut gate2, mut gate3, mut gate4, mut gate5] = gates
        .get_many_mut([
            entities.gate1,
            entities.gate2,
            entities.gate3,
            entities.gate4,
            entities.gate5,
        ])
        .unwrap();
    let switch1 = switches.get(entities.switch1).unwrap();
    let switch2 = switches.get(entities.switch2).unwrap();
    let switch3 = switches.get(entities.switch3).unwrap();
    let switch4 = switches.get(entities.switch4).unwrap();
    let switch5 = switches.get(entities.switch5).unwrap();
    let code1 = codes.get(entities.code1).unwrap();
    let code2 = codes.get(entities.code2).unwrap();
    let code3 = codes.get(entities.code3).unwrap();
    let code4 = codes.get(entities.code4).unwrap();
    let [mut fan1, mut fan2, mut fan3] = fans
        .get_many_mut([entities.fan1, entities.fan2, entities.fan3])
        .unwrap();
    let mut cam1 = sec_cams.get_mut(entities.cam1).unwrap();

    if code1.activated() && !gate1.opened() {
        gate1.open();
    }

    if code2.activated() && !gate3.opened() {
        gate3.open();
    }

    if code3.activated() {
        fan2.spinning = false;
    }

    if code4.activated() && !gate2.opened() {
        gate2.open();
    }

    if switch1.activated() {
        fan1.spinning = false;
    }

    if switch2.activated() && !gate5.opened() {
        gate5.open();
    }

    if switch3.activated() {
        cam1.active = false;
    }

    if switch4.activated() && !gate4.opened() {
        gate4.open();
    }

    if switch5.activated() {
        fan3.spinning = false;
    }

    if cam1.triggered {
        commands.insert_resource(Restart(GameState::Level4));
        game_state.set(GameState::Restart);
    }

    if socket_end.connected() {
        game_state.set(GameState::Level2);
    }
}

fn process_animations(level: Res<Level4>) -> Result<()> {
    Ok(())
}
