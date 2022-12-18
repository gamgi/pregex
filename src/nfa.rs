use crate::ast::{AstNode, Kind};
use crate::distribution::Dist;
use crate::parser::parse;

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub kind: Kind,
    pub outs: Outs,
    pub dist: Option<Dist>,
}

impl State {
    pub fn new(kind: Kind, outs: Outs, dist: Option<Dist>) -> State {
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
    // concatenate asts to single state list (HACK)
    let mut states = Vec::new();
    let mut index: usize = 1; // offset for start_state;
    let mut start: Option<usize> = None;

    for ast in asts {
        // Use index as offset for new NFA fragment
        let end = index + ast.length;
        let nfa_frag = ast_to_frag(ast, index, (Some(end), None), None);
        index = end;
        if start.is_none() {
            start = Some(nfa_frag.start);
        }
        states.extend(nfa_frag.states);
    }

    // Add start state if NFA does not start with one
    let start_state = match states
        .get(0)
        .map(|state| [Kind::Start, Kind::AnchorStart].contains(&state.kind))
        .unwrap_or(false)
    {
        true => vec![],
        false => vec![State::start(start)],
    };
    [start_state, states].concat()
}

#[allow(dead_code)]
pub fn ast_to_nfa(ast: AstNode, index: usize, out: usize) -> Vec<State> {
    let nfa_frag = ast_to_frag(ast, index, (Some(out), None), None);
    nfa_frag.states
}

fn ast_to_frag(ast: AstNode, index: usize, outs: Outs, distribution: Option<Dist>) -> Frag {
    match ast.kind {
        Kind::Alternation(left, right) => {
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
            // left points to start of right and right points to outs
            // left as start
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
        _ => {
            panic!("{} is not allowed in the AST", ast.kind.to_string());
        }
    }
}

fn quantifier_to_frag(
    quantifier: AstNode,
    quantified: AstNode,
    index: usize,
    outs: Outs,
    distribution: Option<Dist>,
) -> Frag {
    match quantifier.kind {
        Kind::Quantifier(c) => {
            match c {
                '?' => {
                    /*
                    quantifier points to left and outs.0
                    left points to outs.0
                    quantifier as start
                    */
                    let quantifier_start = index + quantified.length;
                    let left = ast_to_frag(quantified, index, outs, None);

                    let quantifier = ast_to_frag(
                        quantifier,
                        quantifier_start,
                        (Some(left.start), outs.0),
                        distribution,
                    );

                    Frag {
                        states: [left.states, quantifier.states].concat(),
                        start: quantifier.start,
                        outs,
                    }
                }
                _ => {
                    /*
                    left points to quantifier for * and +
                    quantifier points to left and outs.0
                    left as start for +
                    quantifier as start for rest
                    */
                    let quantifier_start = index + quantified.length;
                    let left = ast_to_frag(quantified, index, (Some(quantifier_start), None), None);

                    let quantifier = ast_to_frag(
                        quantifier,
                        quantifier_start,
                        (Some(index), outs.0),
                        distribution,
                    );
                    let start = match c {
                        '+' => left.start,
                        _ => quantifier.start,
                    };

                    Frag {
                        states: [left.states, quantifier.states].concat(),
                        start,
                        outs,
                    }
                }
            }
        }
        Kind::ExactQuantifier(_) => {
            /*
            left points to quantifier
            quantifier points to left (less) and outs.0 (exact)
            left as start
            */
            let quantifier_start = index + quantified.length;
            let left = ast_to_frag(quantified, index, (Some(quantifier_start), None), None);

            let quantifier = ast_to_frag(
                quantifier,
                quantifier_start,
                (Some(index), outs.0),
                distribution,
            );
            Frag {
                states: [left.states, quantifier.states].concat(),
                start: left.start,
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
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                (Some(0), Some(2)),
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
                        length: 1,
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_star() {
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_plus() {
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
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_exact_quantifier() {
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
                                kind: Kind::ExactQuantifier(2),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            Some(Dist::ExactlyTimes(2)),
                        ),
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
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::ExactlyTimes(2)),
            ),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compile_exact_quantifier_dist() {
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
                                kind: Kind::ExactQuantifier(2),
                            }),
                            Box::new(AstNode {
                                length: 1,
                                kind: Kind::Literal('b'),
                            }),
                            Some(Dist::PGeometric(2, 0.5)),
                        ),
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
            State::new(
                Kind::ExactQuantifier(2),
                (Some(1), Some(3)),
                Some(Dist::PGeometric(2, 0.5)),
            ),
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
                    Some(Dist::PGeometric(0, 0.5)),
                ),
            },
            0,
            1,
        );
        let expected = vec![State::new(
            Kind::Class(vec!['a', 'b', 'c']),
            (Some(1), None),
            Some(Dist::PGeometric(0, 0.5)),
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
                    length: 1,
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
        assert_eq!(result, expected);
    }

    #[test]
    fn test_asts_to_nfa_start_node_special_case_2() {
        let asts = parse("a?b").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(2)),
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
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nfas_to_ast_start_node_special_case_3() {
        let asts = parse("a{2}b").unwrap();
        let result = asts_to_nfa(asts);
        let expected = vec![
            State::start(Some(1)),
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
