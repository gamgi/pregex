#![allow(dead_code, unused_imports, unused_mut, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use crate::state::NfaState;
use log::Level;
use std::collections::HashSet;

pub fn matches(nfa: &Vec<State>, string: &str) -> bool {
    if nfa.len() == 0 {
        return true;
    }
    let mut state = NfaState::new(nfa);
    state.init_state(Some(0), true);

    // step through string
    for c in string.chars() {
        if state.step(c) == 1.0 {
            debug!("match");
            return true;
        }
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
                    kind: Kind::Start,
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Split,
                },
                (Some(2), Some(3)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(4), None),
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
                    kind: Kind::Start,
                },
                (Some(1), None),
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
                (Some(4), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(2), Some(4)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('c'),
                },
                (Some(5), None),
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

    #[test]
    fn test_matches_exact_quantifier() {
        let nfa = vec![
            State {
                kind: Kind::Start,
                outs: (Some(1), None),
            },
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
                    kind: Kind::ExactQuantifier(2),
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
                length: 0,
                kind: Kind::Terminal,
            }),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "aa"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "aba"), false);
        assert_eq!(matches(&nfa, "aab"), true);
        assert_eq!(matches(&nfa, "abb"), false);
        // assert_eq!(matches(&nfa, "aaab"), false);
        assert_eq!(matches(&nfa, "xaab"), false);
        assert_eq!(matches(&nfa, "aabx"), true);
    }
}
