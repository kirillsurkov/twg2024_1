use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;

pub fn generate() -> String {
    let nodes = ["wall", "fan", "gate", "camera"];
    let after = HashMap::from([
        ("input", vec!["tip"]),
        ("switch", vec!["fan", "gate"]),
        ("wall", vec!["fan", "gate"]),
        ("fan", vec!["switch", "input"]),
        ("camera", vec!["switch", "input"]),
        ("gate", vec!["switch", "input"]),
    ]);
    let mut after_done: HashSet<&str> = HashSet::new();

    let mut res = vec![];

    let steps = 4;
    let mut rng = rand::thread_rng();
    let mut cur_node = None;
    let mut i = 0;
    loop {
        if i >= steps && cur_node.is_none() {
            break;
        }

        let node = match cur_node {
            None => {
                if i != 0 {
                    res.push("|");
                }
                i += 1;
                nodes.choose(&mut rng)
            }
            node => node,
        }
        .unwrap();

        res.push(node);
        if after_done.contains(node) {
            after_done.remove(node);
        }

        cur_node = match after.get(node).and_then(|after| after.choose(&mut rng)) {
            Some(after) => match after_done.get(after) {
                Some(_) => None,
                None => Some(after),
            },
            None => None,
        };
    }

    res.reverse();
    res.join(" ")
}