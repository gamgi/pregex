#![allow(dead_code, unused_imports, unused_mut, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::distribution::Dist;
use crate::nfa::State;
use crate::state::NfaState;
use crate::utils;
use log::Level;
use std::collections::HashSet;

pub fn match_p(nfa: &Vec<State>, string: &str) -> Option<f64> {
    if nfa.len() == 0 {
        return Some(1.);
    }
    let mut state = NfaState::new(nfa);
    state.init_state(Some(0), true);

    let tokens = [
        vec![Kind::Start],
        string.chars().map(Kind::Literal).collect::<Vec<_>>(),
        vec![Kind::Terminal],
    ]
    .concat();

    // step through vector of tokens
    let p = utils::find_max(tokens.into_iter().map(|t| state.step(t)));
    Some(p)
}

pub fn matches(nfa: &Vec<State>, string: &str) -> bool {
    const THRESHOLD: f64 = 1.0;
    match match_p(nfa, string) {
        Some(p) => p >= THRESHOLD,
        None => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matches_simple_literal() {
        let nfa = vec![
            State::anchor_start(Some(1)),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "ab"), true);
        assert_eq!(matches(&nfa, "bb"), false);
        assert_eq!(matches(&nfa, "abx"), true);
        assert_eq!(matches(&nfa, "xab"), false);
    }

    #[test]
    fn test_matches_simple_literal_without_start_anchor() {
        let nfa = vec![
            State::start(Some(1)),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "bb"), false);
        assert_eq!(matches(&nfa, "ab"), true);
        assert_eq!(matches(&nfa, "xab"), true);
    }

    #[test]
    fn test_matches_simple_alternation() {
        let nfa = vec![
            State::anchor_start(Some(1)),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "a"), true);
        assert_eq!(matches(&nfa, "ax"), true);
        assert_eq!(matches(&nfa, "bx"), true);
        assert_eq!(matches(&nfa, "xa"), false);
        assert_eq!(matches(&nfa, "xb"), false);
    }

    #[test]
    fn test_matches_simple_dot() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Dot,
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "ab"), true);
        assert_eq!(matches(&nfa, "xb"), true);
        assert_eq!(matches(&nfa, "abx"), true);
        assert_eq!(matches(&nfa, "xab"), false);
    }

    #[test]
    fn test_matches_conditional_first() {
        let nfa = vec![
            State::anchor_start(Some(2)),
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
            State::terminal(),
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
            State::anchor_start(Some(1)),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "axc"), false);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "abbc"), false);
        assert_eq!(matches(&nfa, "ac"), true);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), false);
    }

    #[test]
    fn test_matches_simple_plus() {
        let nfa = vec![
            State::start(Some(1)),
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
                    kind: Kind::Quantifier('+'),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "axc"), false);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "abbc"), true);
        assert_eq!(matches(&nfa, "ac"), false);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), true);
    }

    #[test]
    fn test_matches_simple_star() {
        let nfa = vec![
            State::start(Some(1)),
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
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('*'),
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
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "abc"), true);
        assert_eq!(matches(&nfa, "axc"), false);
        assert_eq!(matches(&nfa, "abbc"), true);
        assert_eq!(matches(&nfa, "ac"), true);
        assert_eq!(matches(&nfa, "abcx"), true);
        assert_eq!(matches(&nfa, "xabc"), true);
    }

    #[test]
    fn test_matches_exact_quantifier() {
        let nfa = vec![
            State::anchor_start(Some(1)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::ExactlyTimes(2)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(4), None),
            ),
            State::terminal(),
        ];
        assert_eq!(matches(&nfa, "a"), false);
        assert_eq!(matches(&nfa, "aa"), false);
        assert_eq!(matches(&nfa, "ab"), false);
        assert_eq!(matches(&nfa, "aba"), false);
        assert_eq!(matches(&nfa, "aab"), true);
        assert_eq!(matches(&nfa, "abb"), false);
        assert_eq!(matches(&nfa, "aaab"), false);
        assert_eq!(matches(&nfa, "xaab"), false);
        assert_eq!(matches(&nfa, "aabx"), true);
    }
}
