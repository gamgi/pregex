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
    // ins: Vec<usize>,
    start: usize,
    // outs: Vec<usize>,
    out: usize,
}

pub fn ast_to_nfa(ast: AstNode, index: usize, out: usize) -> Vec<State> {
    let nfa_frag = ast_to_frag(ast, index, out);
    nfa_frag.states
}

fn ast_to_frag(ast: AstNode, index: usize, out: usize) -> Frag {
    match ast.kind {
        Kind::Concatenation(left, right) => {
            let right = ast_to_frag(*right, index + left.length, out);
            let left = ast_to_frag(*left, index, right.start);
            Frag {
                states: [left.states, right.states].concat(),
                start: left.start,
                out: right.out,
            }
        }
        Kind::Literal(_) => Frag {
            states: vec![State(ast, Some(out), None)],
            start: index,
            out: index + 1,
        },
        Kind::Quantified(c, left) => {
            let quantifier_start = index + left.length;
            let quantifier = ast_to_frag(
                AstNode {
                    length: 1,
                    kind: Kind::Quantifier(c),
                },
                quantifier_start,
                out,
            );
            let left = ast_to_frag(*left, index, out);
            // TODO should be built in ast.rs
            Frag {
                states: [left.states, quantifier.states].concat(),
                start: quantifier_start,
                out: out,
            }
        }
        Kind::Quantifier(_) => Frag {
            states: vec![State(ast, None, Some(out))],
            start: index,
            out: out,
        },
        Kind::Terminal => Frag {
            states: vec![State(ast, None, None)],
            start: index,
            out: index,
        },
        _ => ast_to_frag(ast, index, out),
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
