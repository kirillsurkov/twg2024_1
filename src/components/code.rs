use std::collections::LinkedList;

use bevy::prelude::*;
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings},
    CursorRay,
};
use bevy_rapier2d::geometry::Collider;

use crate::{
    player::{Player, PlayerCollision, ViewController},
    utils::reduce_to_root,
};

use super::loading::Loading;

#[derive(PartialEq)]
enum State {
    Idle,
    Acting,
    InputFinished,
    Success,
}

#[derive(Clone, Debug)]
struct CodeButton {
    entity: Entity,
    number: u8,
    timer: f32,
}

#[derive(Clone)]
struct CodeEntities {
    sensor: Entity,
    screen: Entity,
    segments: [[Entity; 7]; 4],
    buttons: [CodeButton; 10],
}

#[derive(Component)]
pub struct Code {
    entities: Option<CodeEntities>,
    secret: u32,
    input: String,
    is_action_last: bool,
    is_mouse_last: bool,
    finish_timer: f32,
    state: State,
}

impl Code {
    pub fn new(secret: u32) -> Self {
        if secret > 9999 {
            panic!("Secret can only contain 4 digits");
        }
        Self {
            entities: None,
            secret,
            input: String::default(),
            is_action_last: false,
            is_mouse_last: false,
            finish_timer: 0.0,
            state: State::Idle,
        }
    }

    pub fn activated(&self) -> bool {
        self.state == State::Success
    }
}

pub struct CodePlugin;

impl Plugin for CodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(any_with_component::<Loading>()),
                update
                    .run_if(any_with_component::<Code>())
                    .run_if(not(any_with_component::<Loading>())),
            ),
        );
    }
}

fn init(
    mut commands: Commands,
    mut codes: Query<(Entity, &mut Code), With<Loading>>,
    mut visibility: Query<&mut Visibility>,
    parents: Query<&Children>,
    names: Query<&Name>,
    colliders: Query<&Collider>,
    mesh_hs: Query<&Handle<Mesh>>,
) {
    for (entity, mut code) in codes.iter_mut() {
        commands.entity(entity).remove::<Loading>();

        let mut sensor = None;
        let mut screen = None;
        let mut segments = [[None; 7]; 4];
        let mut buttons = [None; 10];

        let mut stack = LinkedList::from([entity]);
        while let Some(current) = stack.pop_back() {
            if let Ok(name) = names.get(current).map(Name::as_str) {
                if name.contains("screen") && mesh_hs.get(current).is_ok() {
                    screen = Some(current);
                } else if name.contains("sensor") && colliders.get(current).is_ok() {
                    sensor = Some(current);
                } else if name.contains("segment_") {
                    let from = name.find("segment_").unwrap() + 8;
                    let digit = name.as_bytes()[from] - 0x30 - 1;
                    let segment = name.as_bytes()[from + 2] - 0x30 - 1;
                    segments[digit as usize][segment as usize] = Some(current);
                    *visibility.get_mut(current).unwrap() = Visibility::Hidden;
                } else if name.contains("btn_") {
                    println!("{name}");
                    let from = name.find("btn_").unwrap() + 4;
                    let number = name.as_bytes()[from] - 0x30;
                    buttons[number as usize] = Some((current, number));
                }
            }
            if let Ok(children) = parents.get(current) {
                stack.extend(children.into_iter());
            }
        }

        code.entities = Some(CodeEntities {
            screen: screen.unwrap(),
            sensor: sensor.unwrap(),
            segments: segments
                .iter()
                .map(|digit| {
                    digit
                        .iter()
                        .map(|segment| segment.unwrap())
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            buttons: buttons
                .iter()
                .map(|btn| {
                    let (entity, number) = btn.unwrap();
                    CodeButton {
                        entity,
                        number,
                        timer: 0.0,
                    }
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        });
    }
}

fn update(
    mut player: ResMut<Player>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut codes: Query<(&mut Code, &Name)>,
    mut transforms: Query<&mut Transform>,
    mut raycast: Raycast,
    mut visibility: Query<&mut Visibility>,
    cursor_ray: Res<CursorRay>,
    time: Res<Time>,
    collisions: Query<&PlayerCollision>,
    transforms_g: Query<&GlobalTransform>,
    children: Query<&Parent>,
    material_hs: Query<&Handle<StandardMaterial>>,
) {
    for (mut code, code_name) in codes.iter_mut() {
        let entities = code.entities.clone().unwrap();

        let inside = collisions
            .iter()
            .find(|c| c.other == entities.sensor)
            .is_some();

        let acted = !code.is_action_last && player.is_action;
        code.is_action_last = player.is_action;

        let clicked = !code.is_mouse_last && player.is_mouse;
        code.is_mouse_last = player.is_mouse;

        match code.state {
            State::Idle => {
                if inside && acted {
                    code.state = State::Acting;

                    let screen = transforms_g.get(entities.screen).unwrap();
                    let from = screen.translation() - 3.0 * screen.forward() - 1.25 * screen.up();
                    let to = screen.translation() - 0.5 * screen.up();
                    player.view_controller = Some(ViewController {
                        name: code_name.to_string(),
                        from,
                        to,
                        hide_player: true,
                    })
                }
            }
            State::Acting => {
                if acted {
                    code.state = State::Idle;
                    player.view_controller = None;
                } else {
                    let mut btn_clicked = None;
                    if let Some(cursor_ray) = **cursor_ray {
                        let buttons = code.entities.as_mut().map(|e| &mut e.buttons).unwrap();
                        let [(entity, _)] =
                            raycast.cast_ray(cursor_ray, &RaycastSettings::default())
                        else {
                            return;
                        };
                        for btn in buttons {
                            if reduce_to_root(&children, *entity, false, |f, p| {
                                f || (p == btn.entity)
                            }) {
                                btn.timer += time.delta_seconds() * 10.0;
                                btn_clicked = Some(btn.number);
                            } else {
                                btn.timer -= time.delta_seconds() * 10.0;
                            }
                            let mut transform = transforms.get_mut(btn.entity).unwrap();
                            btn.timer = btn.timer.max(0.0).min(1.0);
                            let base = 2.0867615;
                            let amount = if clicked { 0.2 } else { 0.1 };
                            transform.translation.z = base - btn.timer * amount;
                        }
                    }
                    if let Some(btn_clicked) = btn_clicked {
                        if clicked {
                            code.input.push((btn_clicked + 0x30) as char);
                        }
                    }
                    if code.input.len() == 4 {
                        code.state = State::InputFinished;
                        code.finish_timer = 0.0;
                    }
                }
            }
            State::InputFinished => {
                code.finish_timer += time.delta_seconds() * 5.0;
                let secret = code.input.parse::<u32>().unwrap();
                let material = materials
                    .get_mut(material_hs.get(entities.screen).unwrap())
                    .unwrap();
                if secret == code.secret {
                    if code.finish_timer >= 1.0 {
                        code.state = State::Success;
                    }
                    material.base_color = Color::rgb_linear(0.0, 1.0, 0.0);
                    material.emissive = Color::rgb_linear(0.5, 10.0, 0.5);
                } else {
                    if code.finish_timer >= 1.0 {
                        code.state = State::Acting;
                        code.input.clear();
                    }
                    material.base_color = Color::rgb_linear(1.0, 0.0, 0.0);
                    material.emissive = Color::rgb_linear(10.0, 0.5, 0.5);
                }
            }
            State::Success => {
                player.view_controller = None;
            }
        }

        for (i, segment) in entities.segments.iter().enumerate() {
            let mask = match code.input.as_bytes().get(i) {
                Some(b'0') => 0b1110111,
                Some(b'1') => 0b0100100,
                Some(b'2') => 0b1011101,
                Some(b'3') => 0b1101101,
                Some(b'4') => 0b0101110,
                Some(b'5') => 0b1101011,
                Some(b'6') => 0b1111011,
                Some(b'7') => 0b0100101,
                Some(b'8') => 0b1111111,
                Some(b'9') => 0b1101111,
                _ => 0 as u8,
            };

            for (i, e) in segment.iter().enumerate() {
                *visibility.get_mut(*e).unwrap() = if (mask >> i) & 1 > 0 {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}
