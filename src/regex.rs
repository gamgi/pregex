use colored::Colorize;

use crate::ast::{AstNode, Kind};
use crate::distribution::Dist;
use crate::nfa::State;
use crate::state;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

type Token = Kind;

static PIXEL_MAP: [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];

fn step_states(
    states: HashMap<usize, f64>,
    counts: &HashMap<usize, u64>,
    token: &Token,
    nfa: &Vec<State>,
) -> HashMap<usize, f64> {
    let mut next: HashMap<usize, f64> = HashMap::new();
    for (state, p) in states.iter() {
        let state = Some(*state);
        let transitions = evaluate_state(state, token, *p, &nfa, &counts, false);
        for transition in transitions {
            if let Transition(Some(out), new_p) = transition {
                let old_p = next.entry(out).or_insert(new_p);
                *old_p = f64::max(*old_p, new_p);
            }
        }
    }
    next
}

fn add_counts(states: &HashMap<usize, f64>, counts: &HashMap<usize, u64>) -> HashMap<usize, u64> {
    let mut updated: HashMap<usize, u64> = counts.clone();
    for (state, p) in states.iter() {
        if *p > 0.0 {
            updated.entry(*state).and_modify(|n| *n += 1).or_insert(1);
        }
    }
    updated
}

pub fn match_likelihood<T>(nfa: &Vec<State>, input: &T) -> Option<f64>
where
    T: Into<Tokens> + Clone,
{
    let mut states = initial_state(nfa, false);
    let mut counts: HashMap<usize, u64> = HashMap::new();
    let tokens: Vec<Token> = input.clone().into().0;

    for token in tokens.iter() {
        debug_print(&nfa, &states, &counts, &token);
        states = step_states(states, &counts, token, nfa);
        counts = add_counts(&states, &counts);
    }
    Some(0.0)
}

fn initial_state(nfa: &Vec<State>, skip_start: bool) -> HashMap<usize, f64> {
    let transitions = evaluate_state(
        Some(0),
        &Kind::Start,
        1.0,
        &nfa,
        &HashMap::new(),
        // for simpler testing (n need for Kind::Start token everywhere)
        !skip_start,
    );
    return transitions
        .into_iter()
        .filter_map(|t| match t {
            Transition(Some(t), p) => Some((t, p)),
            Transition(None, _) => None,
        })
        .collect();
}

fn debug_print(
    nfa: &Vec<State>,
    states: &HashMap<usize, f64>,
    counts: &HashMap<usize, u64>,
    token: &Token,
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

/// Evaluate the state idx against token, return transitions to next states
fn evaluate_state(
    idx: Option<usize>,
    token: &Token,
    p: f64,
    nfa: &Vec<State>,
    counts: &HashMap<usize, u64>,
    is_epsilon: bool,
) -> Vec<Transition> {
    let idx = if let Some(idx) = idx {
        idx
    } else {
        return vec![];
    };

    if let Some(state) = nfa.get(idx) {
        match state.kind {
            Kind::Terminal => {
                // TODO should not add back, right?... idk
                return vec![Transition(Some(idx), p)];
            }
            Kind::Start => {
                if is_epsilon {
                    return vec![Transition(Some(idx), 1.0)];
                }
                return [
                    // Always keep start in states
                    vec![Transition(Some(idx), 1.0)],
                    // Add states along out arrows
                    evaluate_state_outs(state.outs, token, p, nfa, &counts, true),
                ]
                .concat();
            }
            Kind::AnchorStart => {
                if is_epsilon {
                    return vec![Transition(Some(idx), 1.0)];
                }
                if *token == Kind::Start {
                    // Add state along out arrow
                    return evaluate_state(state.outs.0, token, p, nfa, counts, true);
                    // } else {
                    //     return vec![Transition(Some(idx), 1.0)];
                }
            }
            Kind::AnchorEnd => {
                if is_epsilon {
                    return vec![Transition(Some(idx), 1.0)];
                }
                if *token == Kind::Terminal {
                    // Add terminal state
                    return vec![Transition(state.outs.0, 1.0)];
                }
            }
            Kind::Split => {
                return evaluate_state_outs(state.outs, token, p, nfa, &counts, true);
            }
            Kind::Quantifier(_) | Kind::ExactQuantifier(_) => {
                if !is_epsilon {
                    // Direct evaluation is no-op, since state used for counting only
                    return vec![];
                }

                // quantifier now visited current + 1 times
                let n = *counts.get(&idx).unwrap_or(&0) + 1;
                let (p0, p1) = match &state.dist {
                    Some(dist) => dist.evaluate(p, n, false),
                    None => (p, p),
                };

                return [
                    // Always add quantifier state for counting
                    vec![Transition(Some(idx), 1.0)],
                    evaluate_state(state.outs.0, token, p0, nfa, &counts, true),
                    evaluate_state(state.outs.1, token, p1, nfa, &counts, true),
                ]
                .concat();
            }
            Kind::Literal(match_c) => {
                if is_epsilon {
                    return vec![Transition(Some(idx), p)];
                }

                if let Kind::Literal(c) = token {
                    if *c == match_c {
                        return evaluate_state_outs(state.outs, token, p, nfa, &counts, true);
                    }
                }
            }
            _ => {}
        }
    }
    return vec![];
}

/// Helper for evaluating multiple states at once
fn evaluate_state_outs(
    outs: (Option<usize>, Option<usize>),
    token: &Token,
    p: f64,
    nfa: &Vec<State>,
    counts: &HashMap<usize, u64>,
    is_epsilon: bool,
) -> Vec<Transition> {
    [
        evaluate_state(outs.0, token, p, nfa, &counts, is_epsilon),
        evaluate_state(outs.1, token, p, nfa, &counts, is_epsilon),
    ]
    .concat()
}

// struct StateContext(());
#[derive(Debug, Clone, PartialEq)]
struct Transition(Option<usize>, f64);

/// Newtype for vector of input tokens
pub struct Tokens(Vec<Kind>);

impl From<String> for Tokens {
    fn from(s: String) -> Self {
        Self(
            [
                vec![Kind::Start],
                s.chars().map(Kind::Literal).collect::<Vec<_>>(),
                vec![Kind::Terminal],
            ]
            .concat(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_initial_state_start() {
        let nfa = vec![State::start(Some(1)), State::literal('a', (Some(2), None))];
        let states = initial_state(&nfa, false);
        assert_eq!(states, [(0, 1.0)].into());

        let states = initial_state(&nfa, true);
        assert_eq!(states, [(0, 1.0), (1, 1.0)].into());
    }

    #[test]
    fn test_initial_state_anchor_start() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::literal('a', (Some(2), None)),
        ];
        let states = initial_state(&nfa, false);
        assert_eq!(states, [(0, 1.0)].into());

        let states = initial_state(&nfa, true);
        assert_eq!(states, [(1, 1.0)].into());
    }

    #[test]
    fn test_add_counts() {
        let nfa = vec![State::start(Some(1)), State::literal('a', (Some(2), None))];
        let states = initial_state(&nfa, true);

        let counts = add_counts(&states, &HashMap::new());
        assert_eq!(counts, [(0, 1), (1, 1)].into());
    }

    #[test]
    fn test_evaluate_state_literals() {
        let nfa = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::literal('b', (Some(3), None)),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();

        let transitions = evaluate_state(Some(1), &Kind::Literal('a'), 1.0, &nfa, &counts, false);
        assert_eq!(transitions, vec![Transition(Some(2), 1.0)]);

        let transitions = evaluate_state(Some(1), &Kind::Literal('b'), 1.0, &nfa, &counts, false);
        assert_eq!(transitions, vec![]);
    }

    #[test]
    fn test_evaluate_state_geo_quantifier() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::PGeometric(2, 0.5)),
            ),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();

        let transitions = evaluate_state(Some(1), &Kind::Literal('a'), 1.0, &nfa, &counts, false);
        assert_eq!(
            transitions,
            vec![
                Transition(Some(2), 1.0),
                Transition(Some(1), 1.0),
                Transition(Some(3), 0.0)
            ]
        );

        let counts: HashMap<usize, u64> = HashMap::from([(2, 1)]);
        let transitions = evaluate_state(Some(1), &Kind::Literal('a'), 1.0, &nfa, &counts, false);
        assert_eq!(
            transitions,
            vec![
                Transition(Some(2), 1.0),
                Transition(Some(1), 0.5),
                Transition(Some(3), 0.5)
            ]
        );

    }

    #[test]
    fn test_step_states_literals() {
        let nfa = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::literal('b', (Some(3), None)),
            State::terminal(),
        ];
        let counts = HashMap::new();
        let states = initial_state(&nfa, true);
        assert_eq!(states, [(0, 1.0), (1, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(0, 1.0), (1, 1.0), (2, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(0, 1.0), (1, 1.0), (2, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('b'), &nfa);
        assert_eq!(states, [(0, 1.0), (1, 1.0), (3, 1.0)].into());
    }

    #[test]
    fn test_step_states_anchored_literals() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::literal('b', (Some(3), None)),
            State::terminal(),
        ];
        let counts = HashMap::new();
        let states = initial_state(&nfa, true);
        assert_eq!(states, [(1, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(2, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [].into());
    }

    #[test]
    fn test_step_states_alternation() {
        let nfa = vec![
            State::start(Some(1)),
            State::split((Some(2), Some(3))),
            State::literal('a', (Some(4), None)),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        let counts = HashMap::new();
        let states = initial_state(&nfa, true);
        assert_eq!(states, [(0, 1.0), (2, 1.0), (3, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(0, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('b'), &nfa);
        assert_eq!(
            states,
            [(0, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)].into()
        );
    }

    #[test]
    fn test_step_states_exact_quantifier() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::ExactlyTimes(2)),
            ),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        let states = initial_state(&nfa, true);
        assert_eq!(states, [(1, 1.0)].into());

        let counts = add_counts(&states, &HashMap::new());
        assert_eq!(counts, [(1, 1)].into());
        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(1, 1.0), (2, 1.0), (3, 0.0)].into());

        let counts = add_counts(&states, &counts);
        assert_eq!(counts, [(1, 2), (2, 1)].into());
        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        assert_eq!(states, [(1, 0.0), (2, 1.0), (3, 1.0)].into());

        let counts = add_counts(&states, &counts);
        let states = step_states(states, &counts, &Kind::Literal('b'), &nfa);
        assert_eq!(states, [(4, 1.0)].into());
    }

    #[test]
    fn test_step_states_geo_quantifier() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::PGeometric(2, 0.5)),
            ),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        let states = initial_state(&nfa, true);
        let counts = add_counts(&states, &HashMap::new());
        assert_eq!(states, [(1, 1.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        let counts = add_counts(&states, &counts);
        assert_eq!(states, [(1, 1.0), (2, 1.0), (3, 0.0)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        let counts = add_counts(&states, &counts);
        assert_eq!(states, [(1, 0.5), (2, 1.0), (3, 0.5)].into());

        let states = step_states(states, &counts, &Kind::Literal('a'), &nfa);
        let counts = add_counts(&states, &counts);
        assert_eq!(states, [(1, 0.75), (2, 1.0), (3, 0.25)].into());

        let states = step_states(states, &counts, &Kind::Literal('b'), &nfa);
        assert_eq!(states, [(4, 0.25)].into());
    }
}
