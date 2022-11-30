use colored::Colorize;

use crate::{
    ast::{AstNode, Kind},
    distribution::Dist,
    nfa::State,
    regex_state::{evaluate_state, initial_state, terminal_state_p, Token, Tokens, Transition},
    visualization,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

pub fn match_likelihood<T>(nfa: &Vec<State>, input: &T) -> Option<f64>
where
    T: Into<Tokens> + Clone,
{
    let mut states = initial_state(nfa, false);
    let mut counts: HashMap<usize, u64> = HashMap::new();
    let tokens: Vec<Token> = input.clone().into().as_vec();

    for token in tokens.iter() {
        visualization::debug_print(&states, &counts, nfa, &token);
        states = step_states(states, &counts, token, nfa);
        counts = add_counts(&states, &counts);
    }
    return terminal_state_p(&states, &nfa);
}

fn step_states(
    states: HashMap<usize, f64>,
    counts: &HashMap<usize, u64>,
    token: &Token,
    nfa: &Vec<State>,
) -> HashMap<usize, f64> {
    let mut next: HashMap<usize, f64> = HashMap::new();
    for (state, p) in states.iter() {
        let state = Some(*state);
        let transitions = evaluate_state(state, token, *p, &nfa, &counts, &states, false);
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_counts() {
        let nfa = vec![State::start(Some(1)), State::literal('a', (Some(2), None))];
        let states = initial_state(&nfa, true);

        let counts = add_counts(&states, &HashMap::new());
        assert_eq!(counts, [(0, 1), (1, 1)].into());
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
        assert_eq!(states, [(0, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)].into());
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
