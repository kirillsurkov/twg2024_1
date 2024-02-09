use bevy::prelude::*;

pub fn reduce_to_root<F: FnMut(T, Entity) -> T, T>(
    children: &Query<&Parent>,
    from: Entity,
    initial: T,
    mut cb: F,
) -> T {
    let mut acc = initial;
    let mut root = from;
    loop {
        acc = cb(acc, root);
        let Ok(parent) = children.get(root).map(|parent| parent.get()) else {
            break;
        };
        root = parent;
    }
    acc
}
