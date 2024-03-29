use colored::Colorize;

use crate::{
    ast::{AstNode, Kind},
    distribution::Dist,
    nfa::State,
    visualization,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
pub type Token = Kind;

pub fn initial_state(nfa: &Vec<State>, skip_start: bool) -> HashMap<usize, f64> {
    let transitions = evaluate_state(
        Some(0),
        &Kind::Start,
        1.0,
        nfa,
        &HashMap::new(),
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

pub fn terminal_state_p(states: &HashMap<usize, f64>, nfa: &Vec<State>) -> Option<f64> {
    // TODO may not be the terminal state
    let idx_terminal = nfa.len() - 1;
    states.get(&idx_terminal).map(|p| *p)
}

/// Evaluate the state idx against token, return transitions to next states
pub fn evaluate_state(
    idx: Option<usize>,
    token: &Token,
    p: f64,
    nfa: &Vec<State>,
    counts: &HashMap<usize, u64>,
    states: &HashMap<usize, f64>, // for base p for quantifier
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
                    evaluate_state_outs(state.outs, token, p, nfa, counts, states, true),
                ]
                .concat();
            }
            Kind::AnchorStart => {
                if is_epsilon {
                    return vec![Transition(Some(idx), 1.0)];
                }
                if *token == Kind::Start {
                    // Add state along out arrow
                    return evaluate_state(state.outs.0, token, p, nfa, counts, states, true);
                }
            }
            Kind::AnchorEnd => {
                if is_epsilon {
                    return vec![Transition(Some(idx), p)];
                }
                if *token == Kind::Terminal {
                    // Add terminal state
                    return vec![Transition(state.outs.0, p)];
                }
            }
            Kind::Split => {
                return evaluate_state_outs(state.outs, token, p, nfa, counts, states, true);
            }
            Kind::Quantifier(_) | Kind::ExactQuantifier(_) => {
                // NOTE: !
                if !is_epsilon {
                    // Direct evaluation is no-op, since state used for counting only
                    return vec![];
                }

                // quantifier now visited current n times
                let n = *counts.get(&idx).unwrap_or(&0);

                let pb = *states.get(&idx).unwrap_or(&p);
                let (_, p1) = match &state.dist {
                    Some(dist) => dist.pmf_link(token, Some(n), &state.kind, false, false),
                    None => (1., 1.),
                };

                return [
                    // Always add quantifier state for counting & storing base p
                    vec![Transition(Some(idx), pb)],
                    // Re-add outs.0 (quantified) with base p, since quantifier p is applied only to next state
                    evaluate_state(state.outs.0, token, p, nfa, counts, states, true),
                    evaluate_state(state.outs.1, token, p * p1, nfa, counts, states, true),
                ]
                .concat();
            }
            Kind::Dot => {
                if is_epsilon {
                    return vec![Transition(Some(idx), p)];
                }

                return evaluate_state_outs(state.outs, token, p, nfa, counts, states, true);
            }
            Kind::Literal(match_c) => {
                if is_epsilon {
                    return vec![Transition(Some(idx), p)];
                }

                if let Kind::Literal(c) = token {
                    if *c == match_c {
                        return evaluate_state(state.outs.0, token, p, nfa, counts, states, true);
                    }
                }
            }
            Kind::Class(is_negate, ref match_c) => {
                if is_epsilon {
                    return vec![Transition(Some(idx), p)];
                }

                if let Kind::Literal(c) = token {
                    let idx = match match_c.iter().position(|&r| r == *c) {
                        Some(i) => Some(i as u64),
                        None => None,
                    };
                    let (_, p1) = match &state.dist {
                        Some(dist) => dist.pmf_link(token, idx, &state.kind, is_negate, false),
                        None => match (idx, is_negate) {
                            (None, false) => (1., 0.),
                            (None, true) => (1., 1.),
                            (Some(_), false) => (1., 1.),
                            (Some(_), true) => (1., 0.),
                        },
                    };

                    return evaluate_state(state.outs.0, token, p * p1, nfa, counts, states, true);
                }
                return vec![];
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
    states: &HashMap<usize, f64>,
    is_epsilon: bool,
) -> Vec<Transition> {
    [
        evaluate_state(outs.0, token, p, nfa, counts, states, is_epsilon),
        evaluate_state(outs.1, token, p, nfa, counts, states, is_epsilon),
    ]
    .concat()
}

// struct StateContext(());
#[derive(Debug, Clone, PartialEq)]
pub struct Transition(pub Option<usize>, pub f64);

/// Newtype for vector of input tokens
pub struct Tokens(Vec<Kind>);

impl Tokens {
    pub fn as_vec(self) -> Vec<Token> {
        return self.0;
    }
}

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
    use crate::distribution::DistLink;

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
    fn test_evaluate_state_literals() {
        let nfa = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::literal('b', (Some(3), None)),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();
        let states: HashMap<usize, f64> = HashMap::new();

        let transitions = evaluate_state(
            Some(1),
            &Kind::Literal('a'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(transitions, vec![Transition(Some(2), 1.0)]);

        let transitions = evaluate_state(
            Some(1),
            &Kind::Literal('b'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(transitions, vec![]);
    }

    #[test]
    fn test_evaluate_state_dot() {
        let nfa = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::dot((Some(3), None)),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();
        let states: HashMap<usize, f64> = HashMap::new();

        let transitions = evaluate_state(
            Some(1),
            &Kind::Literal('a'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(transitions, vec![Transition(Some(2), 1.0)]);

        let transitions = evaluate_state(
            Some(1),
            &Kind::Literal('b'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(transitions, vec![]);
    }

    #[test]
    fn test_evaluate_state_geo_quantifier() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(2), Some(3)),
                Some(DistLink::Counted(Dist::PGeometric(2, u64::MAX, 0.5))),
            ),
            State::literal('a', (Some(1), None)),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();
        let states: HashMap<usize, f64> = HashMap::new();

        let transitions = evaluate_state(
            Some(2),
            &Kind::Literal('a'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(
            transitions,
            vec![
                Transition(Some(1), 1.0), // Store p_entry
                Transition(Some(2), 1.0),
                Transition(Some(3), 0.0)
            ]
        );

        // TODO more steps
    }

    #[test]
    fn test_evaluate_state_geo_class() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::new(
                Kind::Class(false, vec!['a', 'b', 'c']),
                (Some(2), None),
                Some(DistLink::Indexed(Dist::PGeometric(0, u64::MAX, 0.5))),
            ),
            State::terminal(),
        ];
        let counts: HashMap<usize, u64> = HashMap::new();
        let states: HashMap<usize, f64> = HashMap::new();

        let transitions = evaluate_state(
            Some(1),
            &Kind::Literal('a'),
            1.0,
            &nfa,
            &counts,
            &states,
            false,
        );
        assert_eq!(
            transitions,
            // NOTE: The character class is not a quantifier
            vec![Transition(Some(2), 0.5),]
        );
    }
}
