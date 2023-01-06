use crate::ast::{AstNode, Kind};
use crate::distribution::{Dist, DistLink};
use crate::parser::parse;
use itertools::Itertools;

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub kind: Kind,
    pub outs: Outs,
    pub dist: Option<DistLink>,
}

impl State {
    pub fn new(kind: Kind, outs: Outs, dist: Option<DistLink>) -> State {
        State { kind, outs, dist }
    }
    pub fn from(node: AstNode, outs: Outs) -> State {
        State {
            kind: node.kind,
            outs,
            dist: None,
        }
    }
    pub fn start(start: Option<usize>) -> State {
        State {
            kind: Kind::Start,
            outs: (start, None),
            dist: None,
        }
    }
    #[allow(dead_code)]
    pub fn anchor_start(start: Option<usize>) -> State {
        State {
            kind: Kind::AnchorStart,
            outs: (start, None),
            dist: None,
        }
    }
    #[allow(dead_code)]
    pub fn anchor_end(end: Option<usize>) -> State {
        State {
            kind: Kind::AnchorEnd,
            outs: (end, None),
            dist: None,
        }
    }
    pub fn terminal() -> State {
        State {
            kind: Kind::Terminal,
            outs: (None, None),
            dist: None,
        }
    }
    #[allow(dead_code)]
    pub fn literal(char: char, outs: Outs) -> State {
        State {
            kind: Kind::Literal(char),
            outs,
            dist: None,
        }
    }
    #[allow(dead_code)]
    pub fn split(outs: Outs) -> State {
        State {
            kind: Kind::Split,
            outs,
            dist: None,
        }
    }
    #[allow(dead_code)]
    pub fn dot(outs: Outs) -> State {
        State {
            kind: Kind::Dot,
            outs,
            dist: None,
        }
    }
}

type Outs = (Option<usize>, Option<usize>);

#[derive(Debug)]
struct Frag {
    states: Vec<State>,
    start: usize,
    outs: Outs,
}

/// Compile a list of abstract syntax trees into a NFA.
///
/// The compilation first parses the AST(s) into fragments which
/// represent partial NFA states following Thompson [1968].
/// The fragments are then joined to form the final NFA.
/// The NFA is initialized with a Start state.
pub fn asts_to_nfa(asts: Vec<AstNode>) -> Vec<State> {
    let mut states = Vec::new();
    let mut start: usize = 1; // offset for start_state;
    let mut first_start: Option<usize> = None;

    for ast in asts {
        let end = start + ast.length;
        let nfa_frag = ast_to_frag(ast, start, (Some(end), None), None);
        start = end;

        if first_start.is_none() {
            first_start = Some(nfa_frag.start);
        }
        states.extend(nfa_frag.states);
    }

    // Add start state if NFA does not start with one
    let prepend_states = states
        .drain(0..1)
        .flat_map(|s| match s.kind {
            Kind::AnchorStart | Kind::Start => vec![s],
            _ => vec![State::start(first_start), s],
        })
        .collect();

    [prepend_states, states].concat()
}

#[allow(dead_code)]
pub fn ast_to_nfa(ast: AstNode, index: usize, out: usize) -> Vec<State> {
    ast_to_frag(ast, index, (Some(out), None), None).states
}

fn ast_to_frag(ast: AstNode, index: usize, outs: Outs, distribution: Option<DistLink>) -> Frag {
    match ast.kind {
        Kind::Alternation(left, right) => {
            /*
                      ┌──► left ───┐
                ──► split         outs ──►
                      └──► right ──┘
            */
            let right = ast_to_frag(*right, index + left.length + 1, outs, None);
            let left = ast_to_frag(*left, index + 1, outs, None);
            let split = ast_to_frag(
                AstNode {
                    length: 1,
                    kind: Kind::Split,
                    // TODO use distribution
                },
                index,
                (Some(left.start), Some(right.start)),
                None,
            );
            Frag {
                states: [split.states, left.states, right.states].concat(),
                start: split.start,
                outs,
            }
        }
        Kind::AnchorEnd => Frag {
            states: vec![State::new(Kind::AnchorEnd, outs, None)],
            start: index,
            outs,
        },
        Kind::AnchorStart => Frag {
            states: vec![State::new(Kind::AnchorStart, outs, None)],
            start: index,
            outs,
        },
        Kind::Concatenation(left, right) => {
            /*
                ──► left ──► right ──► outs
            */
            let right = ast_to_frag(*right, index + left.length, outs, None);
            let left = ast_to_frag(*left, index, (Some(right.start), None), None);
            Frag {
                states: [left.states, right.states].concat(),
                start: left.start,
                outs: right.outs,
            }
        }
        Kind::Literal(_) | Kind::Dot => Frag {
            // literal points to outs
            // literal as start
            states: vec![State::from(ast, outs)],
            start: index,
            outs,
        },
        Kind::Classified(class, distribution) => Frag {
            // classifier points to class
            // classifier as start
            states: vec![State::new(class.kind, outs, distribution)],
            start: index,
            outs,
        },
        Kind::Class(_) => Frag {
            // class points to outs
            // class as start
            states: vec![State::new(ast.kind, outs, distribution)],
            start: index,
            outs,
        },
        Kind::Quantified(quantifier, quantified, distribution) => {
            quantifier_to_frag(*quantifier, *quantified, index, outs, distribution)
        }
        Kind::Quantifier(_) | Kind::ExactQuantifier(_) => Frag {
            // quantifier points to outs
            // quantifier as start
            states: vec![State::new(ast.kind, outs, distribution)],
            start: index,
            outs,
        },
        Kind::Start => Frag {
            // start points to outs.
            // start as start
            states: vec![State::new(Kind::Start, outs, None)],
            start: index,
            outs,
        },
        Kind::Terminal => Frag {
            // terminal points to none
            // terminal as start
            states: vec![State::terminal()],
            start: index,
            outs: (None, None),
        },
        Kind::Split => Frag {
            // split points to left and right
            // split as start
            states: vec![State::from(ast, outs)],
            start: index,
            outs,
        },
    }
}

fn quantifier_to_frag(
    quantifier: AstNode,
    quantified: AstNode,
    index: usize,
    outs: Outs,
    distribution: Option<DistLink>,
) -> Frag {
    match quantifier.kind {
        Kind::Quantifier(c) => {
            match c {
                '?' => {
                    /*
                        ──► quantifier ──► quantified ──► outs
                                └───────────────────────► outs.0
                    */
                    let quantified_start = index + quantifier.length;
                    let quantified = ast_to_frag(quantified, quantified_start, outs, None);

                    let quantifier = ast_to_frag(
                        quantifier,
                        index,
                        (Some(quantified.start), outs.0),
                        distribution,
                    );

                    Frag {
                        states: [quantifier.states, quantified.states].concat(),
                        start: index,
                        outs,
                    }
                }
                _ => {
                    /*
                                ┌───────◄───────┐
                        ──► quantifier ──► quantified
                                └───────────────────────► outs.0
                    */
                    let quantified_start = index + quantifier.length;
                    let quantified =
                        ast_to_frag(quantified, quantified_start, (Some(index), None), None);

                    let quantifier = ast_to_frag(
                        quantifier,
                        index,
                        (Some(quantified_start), outs.0),
                        distribution,
                    );

                    Frag {
                        states: [quantifier.states, quantified.states].concat(),
                        start: index,
                        outs,
                    }
                }
            }
        }
        Kind::ExactQuantifier(_) => {
            /*
                        ┌───────◄───────┐
                ──► quantifier ──► quantified
                        └───────────────────────► outs.0
            */
            let quantified_start = index + quantifier.length;
            let quantified = ast_to_frag(quantified, quantified_start, (Some(index), None), None);

            let quantifier = ast_to_frag(
                quantifier,
                index,
                (Some(quantified.start), outs.0),
                distribution,
            );

            Frag {
                states: [quantifier.states, quantified.states].concat(),
                start: index,
                outs,
            }
        }
        _ => {
            panic!("{} is not a valid quantifier", quantifier.kind);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let result = ast_to_nfa(
            AstNode {
                length: 0,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('b'),
                    }),
                ),
            },
            0,
            2,
        );
        let expected = vec![
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_alternation() {
        let result = ast_to_nfa(
            AstNode {
                length: 2,
                kind: Kind::Alternation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('b'),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_conditional_first() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Quantifier('?'),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('a'),
                            }),
                            None,
                        ),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('b'),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(1), Some(2)),
            ),
            State::literal('a', (Some(2), None)),
            State::literal('b', (Some(3), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_conditional() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Quantifier('?'),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            None,
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(2), Some(3)),
            ),
            State::literal('b', (Some(3), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_quantifier_star() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Quantifier('*'),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            None,
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('*'),
                },
                (Some(2), Some(3)),
            ),
            State::literal('b', (Some(1), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_quantifier_plus() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Quantifier('+'),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            None,
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('+'),
                },
                (Some(2), Some(3)),
            ),
            State::literal('b', (Some(1), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_quantifier_exact() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::ExactQuantifier(2),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            Some(Dist::ExactlyTimes(2).count()),
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(2), Some(3)),
                Some(Dist::ExactlyTimes(2).count()),
            ),
            State::literal('b', (Some(1), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_quantifier_exact_dist() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 2,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::ExactQuantifier(2),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            Some(Dist::PGeometric(2, u64::MAX, 0.5).count()),
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(2), Some(3)),
                Some(Dist::PGeometric(2, u64::MAX, 0.5).count()),
            ),
            State::literal('b', (Some(1), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_quantifier_exact_zero_dist() {
        let result = ast_to_nfa(
            AstNode {
                length: 3,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Quantified(
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::ExactQuantifier(0),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            Some(Dist::PGeometric(1, u64::MAX, 0.5).count()),
                        ),
                    }),
                ),
            },
            0,
            3,
        );
        let expected = vec![
            State::literal('a', (Some(1), None)),
            State::new(
                Kind::ExactQuantifier(0),
                (Some(2), Some(3)),
                Some(Dist::PGeometric(1, u64::MAX, 0.5).count()),
            ),
            State::literal('b', (Some(1), None)),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_exact_class() {
        let result = ast_to_nfa(
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(vec!['a', 'b', 'c']),
                    }),
                    None,
                ),
            },
            0,
            1,
        );
        let expected = vec![State::from(
            AstNode {
                length: 1,
                kind: Kind::Class(vec!['a', 'b', 'c']),
            },
            (Some(1), None),
        )];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_exact_class_dist() {
        let result = ast_to_nfa(
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(vec!['a', 'b', 'c']),
                    }),
                    Some(Dist::PGeometric(0, u64::MAX, 0.5).count()),
                ),
            },
            0,
            1,
        );
        let expected = vec![State::new(
            Kind::Class(vec!['a', 'b', 'c']),
            (Some(1), None),
            Some(Dist::PGeometric(0, u64::MAX, 0.5).count()),
        )];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_ast_to_frag_outs() {
        let result = ast_to_frag(
            AstNode {
                length: 0,
                kind: Kind::Concatenation(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('b'),
                    }),
                ),
            },
            0,
            (Some(2), None),
            None,
        );

        assert_eq!(result.outs, (Some(2), None));
    }

    #[test]
    fn test_asts_to_nfa() {
        let first = AstNode {
            length: 3,
            kind: Kind::Concatenation(
                Box::new(AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                }),
                Box::new(AstNode {
                    length: 2,
                    kind: Kind::Quantified(
                        Box::new(AstNode {
                            length: 1,
                            kind: Kind::Quantifier('*'),
                        }),
                        Box::new(AstNode {
                            length: 1,
                            kind: Kind::Literal('b'),
                        }),
                        None,
                    ),
                }),
            ),
        };
        let second = AstNode {
            length: 0,
            kind: Kind::Terminal,
        };
        let expected = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('*'),
                },
                (Some(3), Some(4)),
            ),
            State::literal('b', (Some(2), None)),
            State::terminal(),
        ];

        let result = asts_to_nfa(vec![first, second]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_with_anchor_start() {
        let first = AstNode {
            length: 0,
            kind: Kind::AnchorStart,
        };
        let second = AstNode {
            length: 2,
            kind: Kind::Concatenation(
                Box::new(AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                }),
                Box::new(AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                }),
            ),
        };
        let expected = vec![
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
        ];

        let result = asts_to_nfa(vec![first, second]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_with_anchor_end() {
        let first = AstNode {
            length: 2,
            kind: Kind::Concatenation(
                Box::new(AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                }),
                Box::new(AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                }),
            ),
        };
        let second = AstNode {
            length: 1,
            kind: Kind::AnchorEnd,
        };
        let expected = vec![
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
            State::anchor_end(Some(4)),
        ];

        let result = asts_to_nfa(vec![first, second]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_start_node_special_case_1() {
        let asts = parse("ab?c").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(3), Some(4)),
            ),
            State::literal('b', (Some(4), None)),
            State::literal('c', (Some(5), None)),
            State::terminal(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_start_node_special_case_2() {
        let asts = parse("ab{0}c").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(1)),
            State::literal('a', (Some(2), None)),
            State {
                kind: Kind::ExactQuantifier(0),
                outs: (Some(3), Some(4)),
                dist: Some(Dist::ExactlyTimes(0).count()),
            },
            State::literal('b', (Some(2), None)),
            State::literal('c', (Some(5), None)),
            State::terminal(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_start_node_special_case_3() {
        let asts = parse("a?b").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(1)),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(2), Some(3)),
            ),
            State::literal('a', (Some(3), None)),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nfas_to_ast_start_node_special_case_4() {
        let asts = parse("a{2}b").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(1)),
            State::new(
                Kind::ExactQuantifier(2),
                (Some(2), Some(3)),
                Some(Dist::ExactlyTimes(2).count()),
            ),
            State::literal('a', (Some(1), None)),
            State::literal('b', (Some(4), None)),
            State::terminal(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_with_anchor_start_2() {
        let asts = parse("^ab").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::AnchorStart,
                },
                (Some(1), None),
            ),
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
        assert_eq!(result, expected);
    }
}
