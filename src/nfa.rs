use crate::ast::{AstNode, Kind};
use crate::distribution::{Dist, StateParams};
use crate::parser::parse;

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub kind: Kind,
    pub outs: Outs,
    pub params: Option<Dist>,
}

impl State {
    pub fn new(kind: Kind, outs: Outs, params: Option<Dist>) -> State {
        State { kind, outs, params }
    }
    pub fn from(node: AstNode, outs: Outs) -> State {
        State {
            kind: node.kind,
            outs: outs,
            params: None,
        }
    }
    pub fn start(start: Option<usize>) -> State {
        State {
            kind: Kind::Start,
            outs: (start, None),
            params: None,
        }
    }
    pub fn terminal() -> State {
        State {
            kind: Kind::Terminal,
            outs: (None, None),
            params: None,
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
pub fn asts_to_nfa(asts: Vec<AstNode>) -> Vec<State> {
    // concatenate asts to single state list (HACK)
    let mut states = Vec::new();
    let mut index: usize = 1; // offset for start_state;
    let mut start: Option<usize> = None;

    for ast in asts {
        let end = index + ast.length;
        let nfa_frag = ast_to_frag(ast, index, (Some(end), None), None);
        index = nfa_frag.states.len();
        if let None = start {
            start = Some(nfa_frag.start);
        }
        states.extend(nfa_frag.states);
    }
    let start_state = vec![State::start(start)];
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
                outs: outs,
            }
        }
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
        Kind::Literal(_) => Frag {
            // literal points to outs
            // literal as start
            states: vec![State::from(ast, outs)],
            start: index,
            outs: outs,
        },
        Kind::Quantified(quantifier, quantified, distribution) => {
            quantifier_to_frag(*quantifier, *quantified, index, outs, distribution)
        }
        Kind::Quantifier(_) | Kind::ExactQuantifier(_) => Frag {
            // quantifier points to outs
            // quantifier as start
            states: vec![State::new(ast.kind, outs, distribution)],
            start: index,
            outs: outs,
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
            outs: outs,
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

                    return Frag {
                        states: [left.states, quantifier.states].concat(),
                        start: quantifier.start,
                        outs: outs,
                    };
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

                    return Frag {
                        states: [left.states, quantifier.states].concat(),
                        start: start,
                        outs: outs,
                    };
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
            return Frag {
                states: [left.states, quantifier.states].concat(),
                start: left.start,
                outs: outs,
            };
        }
        _ => {
            panic!("{} is not a valid quantifier", quantifier.kind.to_string());
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
                            Some(Dist::PGeometric(0.5)),
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
                Some(Dist::PGeometric(0.5)),
            ),
        ];
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
    fn test_nfas_to_ast() {
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
            length: 1,
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
    fn test_nfas_to_ast2() {
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
    fn test_nfas_to_ast3() {
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
    fn test_nfas_to_ast4() {
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
}
