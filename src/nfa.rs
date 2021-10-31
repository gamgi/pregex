use crate::ast::{AstNode, Kind};

#[derive(Debug, PartialEq, Clone)]
pub struct State(pub AstNode, pub Option<usize>, pub Option<usize>);

impl State {
    fn new(c: AstNode) -> State {
        State(c, None, None)
    }
}

#[derive(Debug)]
struct Frag {
    states: Vec<State>,
    start: usize,
    outs: (usize, Option<usize>),
}

pub fn ast_to_nfa(ast: AstNode, index: usize, out: usize) -> Vec<State> {
    let nfa_frag = ast_to_frag(ast, index, (out, None));
    nfa_frag.states
}

fn ast_to_frag(ast: AstNode, index: usize, outs: (usize, Option<usize>)) -> Frag {
    match ast.kind {
        Kind::Concatenation(left, right) => {
            let right = ast_to_frag(*right, index + left.length, outs);
            let left = ast_to_frag(*left, index, (right.start, None));
            Frag {
                states: [left.states, right.states].concat(),
                start: left.start,
                outs: right.outs,
            }
        }
        Kind::Literal(_) => Frag {
            states: vec![State(ast, Some(outs.0), None)],
            start: index,
            outs: (index + 1, None),
        },
        Kind::Quantified(c, left) => {
            let quantifier_start = index + left.length;
            let quantifier = ast_to_frag(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier(c),
                },
                quantifier_start,
                (index, Some(outs.0)),
            );
            let left = ast_to_frag(*left, index, outs);
            // TODO should be built in ast.rs
            Frag {
                states: [left.states, quantifier.states].concat(),
                start: quantifier_start,
                outs: outs,
            }
        }
        Kind::Quantifier(_) => Frag {
            states: vec![State(ast, Some(outs.0), outs.1)],
            start: index,
            outs: outs,
        },
        Kind::Terminal => Frag {
            states: vec![State(ast, None, None)],
            start: index,
            outs: (index, None),
        },
        _ => ast_to_frag(ast, index, outs),
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
            State(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                Some(1),
                None,
            ),
            State(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                Some(2),
                None,
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
                            '?',
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
            State(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                Some(2),
                None,
            ),
            State(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                Some(3),
                None,
            ),
            State(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier('?'),
                },
                Some(1),
                Some(3),
            ),
        ];
        assert_eq!(result, expected);
    }
}
