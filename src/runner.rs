#![allow(dead_code, unused_imports, unused_mut, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use log::Level;
use std::collections::HashSet;

fn add_state(
    add_idx: Option<usize>,
    nfa: &Vec<State>,
    visited: &mut HashSet<usize>,
    current_states: &mut Vec<State>,
    force: bool,
) {
    if let Some(i) = add_idx {
        // add i to visited and skip adding if (exists and not forced)
        if !visited.insert(i) && !force {
            debug!("    skip {}", nfa[i].node.to_string());
            return;
        }
        let state = &nfa[i];
        if let Kind::Quantifier(c) = state.node.kind {
            // follow outs of quantifier
            debug!("  split {:?} and {:?}", state.outs.0, state.outs.1);
            add_state(state.outs.0, nfa, visited, current_states, force);
            add_state(state.outs.1, nfa, visited, current_states, force);
            return;
        } else if let Kind::Start = state.node.kind {
            // follow out of start
            debug!("  start at {:?}", state.outs.0);
            add_state(state.outs.0, nfa, visited, current_states, force);
            return;
        } else {
            // add state
            debug!("    add  {}", state.node.to_string());
            current_states.push(state.clone());
        }
    }
}

fn step(
    c: char,
    nfa: &Vec<State>,
    visited: &mut HashSet<usize>,
    current_states: Vec<State>,
) -> Vec<State> {
    let mut new_states = Vec::new();
    if log_enabled!(Level::Debug) {
        debug!("step {}", c);
        debug!(
            "  current_states {:?}",
            current_states
                .iter()
                .map(|s| s.node.to_string())
                .collect::<Vec<String>>()
        );
    }

    for state in current_states.iter() {
        if let Kind::Terminal = state.node.kind {
            // end
            debug!("  match terminal");
            add_state(state.outs.0, nfa, visited, &mut new_states, false);
        } else if let Kind::Split = state.node.kind {
            // split
            debug!("  match split");
            add_state(state.outs.0, nfa, visited, &mut new_states, false);
            add_state(state.outs.1, nfa, visited, &mut new_states, false);
        } else if state.node.to_string() == c.to_string() {
            // match
            debug!("  match {} add {:?}", c, state.outs.0);
            add_state(state.outs.0, nfa, visited, &mut new_states, true);
        } else {
            debug!("  {} != {}", state.node.to_string(), c.to_string());
        }
    }
    new_states
}

pub fn matches(nfa: &Vec<State>, string: &str) -> bool {
    if nfa.len() == 0 {
        return true;
    }
    let end = nfa.len() - 1;
    let mut visited: HashSet<usize> = HashSet::new();
    let mut current_states = Vec::new();
    add_state(Some(0), nfa, &mut visited, &mut current_states, true);
    // visited.drain(); // ignore start state visists
    for c in string.chars() {
        current_states = step(c, nfa, &mut visited, current_states);
        if visited.contains(&end) {
            return true;
        }
        visited.drain();
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matches_simple_literal() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(2), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "ab"), true);
        assert_eq!(matches(&nfa, "bb"), false);
        assert_eq!(matches(&nfa, "abx"), true);
        assert_eq!(matches(&nfa, "xab"), false);
    }

    #[test]
    fn test_matches_simple_alternation() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Split,
                },
                (Some(1), Some(2)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(3), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(3), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), true);
        assert_eq!(matches(&nfa, "ax"), true);
        assert_eq!(matches(&nfa, "bx"), true);
        assert_eq!(matches(&nfa, "xa"), false);
        assert_eq!(matches(&nfa, "xb"), false);
    }

    #[test]
    fn test_matches_conditional_first() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Start,
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(3), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(1), Some(3)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(4), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "b"), true);
        assert_eq!(matches(&nfa, "ab"), true);
        assert_eq!(matches(&nfa, "bx"), true);
        assert_eq!(matches(&nfa, "xa"), false);
        assert_eq!(matches(&nfa, "xb"), false);
    }

    #[test]
    fn test_matches_simple_conditional() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(3), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(1), Some(3)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('c'),
                },
                (Some(4), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "abbc"), false);
        assert_eq!(matches(&nfa, "ac"), true);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), false);
    }

    #[test]
    fn test_matches_simple_plus() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('+'),
                },
                (Some(1), Some(3)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('c'),
                },
                (Some(4), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "abbc"), true);
        assert_eq!(matches(&nfa, "ac"), false);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), false);
    }

    #[test]
    fn test_matches_simple_star() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('*'),
                },
                (Some(1), Some(3)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('c'),
                },
                (Some(4), None),
            ),
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "abbc"), true);
        assert_eq!(matches(&nfa, "ac"), true);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), false);
    }
}
