use crate::ast::{AstNode, Kind};

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub node: AstNode,
    pub outs: Outs,
}

impl State {
    pub fn new(node: AstNode) -> State {
        State {
            node: node,
            outs: (None, None),
        }
    }
    pub fn from(node: AstNode, outs: Outs) -> State {
        State {
            node: node,
            outs: outs,
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

pub fn asts_to_nfa(asts: Vec<AstNode>) -> Vec<State> {
    // concatenate asts to single state list (HACK)
    let mut states = Vec::new();
    let mut start: usize = 0;
    for ast in asts {
        let end = start + ast.length;
        let nfa_frag = ast_to_frag(ast, start, (Some(end), None));
        if let Some(len) = nfa_frag.outs.0 {
            start = len;
        }
        states.extend(nfa_frag.states);
    }
    states
}

pub fn ast_to_nfa(ast: AstNode, index: usize, out: usize) -> Vec<State> {
    let nfa_frag = ast_to_frag(ast, index, (Some(out), None));
    nfa_frag.states
}

fn ast_to_frag(ast: AstNode, index: usize, outs: Outs) -> Frag {
    match ast.kind {
        Kind::Concatenation(left, right) => {
            // left points to start of right and right points to outs
            // left as start
            let right = ast_to_frag(*right, index + left.length, outs);
            let left = ast_to_frag(*left, index, (Some(right.start), None));
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
        Kind::Quantified(quantifier, left) => {
            // left points to
            // - outs for ?
            // - quantifier for *
            // and quantifier points to left and outs.0
            // quantifier as start
            let quantifier_start = index + left.length;
            let left = if let Kind::Quantifier(c) = quantifier.kind {
                let left_outs = match c {
                    '*' => (Some(quantifier_start), None),
                    _ => outs,
                };
                ast_to_frag(*left, index, left_outs)
            } else {
                panic!("invalid quantifier {}", quantifier);
            };

            let quantifier = ast_to_frag(*quantifier, quantifier_start, (Some(index), outs.0));
            Frag {
                states: [left.states, quantifier.states].concat(),
                start: quantifier_start,
                outs: outs,
            }
        }
        Kind::Quantifier(_) => Frag {
            // quantifier points to outs
            // quantifier as start
            states: vec![State::from(ast, outs)],
            start: index,
            outs: outs,
        },
        Kind::Terminal => Frag {
            // terminal points to none
            // terminal as start
            states: vec![State::new(ast)],
            start: index,
            outs: (None, None),
        },
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
                    ),
                }),
            ),
        };
        let second = AstNode {
            length: 1,
            kind: Kind::Terminal,
        };
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
            State::new(AstNode {
                length: 1,
                kind: Kind::Terminal,
            }),
        ];

        let result = asts_to_nfa(vec![first, second]);
        assert_eq!(result, expected);
    }
}
