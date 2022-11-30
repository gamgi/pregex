use colored::Colorize;

use crate::ast::{AstNode, Kind};
use crate::distribution::Dist;
use crate::nfa::State;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

static PIXEL_MAP: [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];

pub fn debug_print(
    states: &HashMap<usize, f64>,
    counts: &HashMap<usize, u64>,
    nfa: &Vec<State>,
    token: &Kind,
) {
    for (i, _) in nfa.iter().enumerate() {
        // let (p, n) = match states.get(&i) {
        //     Some((p, n)) => (f64::clamp(p * 4.0, 0., 4.) as usize, *n as u8),
        //     None => (0, 1),
        // };
        let n = match counts.get(&i) {
            Some(n) => usize::clamp(*n as usize, 0, 4),
            None => 0,
        };
        let s = String::from(char::from_u32(0x2800 + PIXEL_MAP[n] as u32).unwrap());
        print!("{}", s);
    }
    println!("");
    for (i, state) in nfa.iter().enumerate() {
        let c = match states.get(&i) {
            Some(p) => (
                u8::clamp((p * 255.0) as u8, 25, 255),
                0,
                u8::clamp((p * 255.0) as u8, 25, 255),
            ),
            None => (50, 50, 50),
        };
        print!("{}", state.kind.to_string().truecolor(c.0, c.1, c.2));
    }
    print!(" ");
    print!("{:5} ", token);

    let probs = states
        .keys()
        .sorted()
        .map(|i| format!("p({})={:?}", nfa[*i].kind, states[i]))
        .collect::<Vec<String>>();
    println!("{}", probs.join(" ").cyan());
}
