use crate::ast::{build_ast_from_expr, AstNode, Kind};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct RegexParser;

pub fn parse(source: &str) -> std::result::Result<Vec<AstNode>, pest::error::Error<Rule>> {
    let mut ast = Vec::new();
    let pairs = RegexParser::parse(Rule::Regex, source)?;

    for pair in pairs {
        if let Rule::EOI = pair.as_rule() {
            ast.push(AstNode {
                length: 0,
                kind: Kind::Terminal,
            });
        } else {
            let node = build_ast_from_expr(pair);
            ast.push(node);
        }
    }
    Ok(ast)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::distribution::{Dist, DistLink};

    fn ast_as_str(asts: Vec<AstNode>) -> String {
        asts.into_iter()
            .map(|ast| ast.to_string())
            .collect::<Vec<String>>()
            .join("")
    }

    #[test]
    fn test_parser_single_ast() {
        let result = parse("a").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Literal('a'),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_alternation_ast() {
        let result = parse("a|b").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 3, // space for split
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
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_conditional_ast() {
        let result = parse("a?").unwrap_or_default();
        let expected = vec![
            AstNode {
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
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_conditional_first_ast() {
        let result = parse("a?b").unwrap_or_default();
        let expected = vec![
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
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_quantifier_ast() {
        let result = parse("a{2}").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 2,
                kind: Kind::Quantified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::ExactQuantifier(2),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Some(Dist::ExactlyTimes(2).count()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_quantifier_dist_ast() {
        let result = parse("a{2~Geo(0.5)}").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 2,
                kind: Kind::Quantified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::ExactQuantifier(2),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    Some(Dist::PGeometric(2, u64::MAX, 0.5).count()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_zero_quantifier_dist_ast() {
        let result = parse("a{0~Const}").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 2,
                kind: Kind::Quantified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::ExactQuantifier(0),
                    }),
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Literal('a'),
                    }),
                    // TODO maybe have n?min and n?max here
                    Some(Dist::Constant(0, 0, 1.0).count()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_ast() {
        let result = parse("[abc]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Class(false, vec!['a', 'b', 'c']),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_indexed_dist_ast() {
        let result = parse("[abc~Geo(0.5)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b', 'c']),
                    }),
                    Some(Dist::PGeometric(0, u64::MAX, 0.5).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_dist_ast() {
        let result = parse("[ab~Cat(a=0.7,b=0.2)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.10000000000000009, 0.7, 0.2]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_dist_rest_ast() {
        let result = parse("[abc~Cat(a=0.7)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b', 'c']),
                    }),
                    Some(
                        Dist::Categorical(vec![0.0, 0.7, 0.15000000000000002, 0.15000000000000002])
                            .index(),
                    ),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_class_dist_ast() {
        let result = parse("[\\d~Cat(1=0.7,2=0.3)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(
                            false,
                            vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                        ),
                    }),
                    Some(
                        Dist::Categorical(vec![
                            0.0, 0.0, 0.7, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                        ])
                        .index(),
                    ),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_dot_ast() {
        let result = parse("[ab~Cat(a=0.7,.=0.1)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.1, 0.7, 0.20000000000000007]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_dot_ast_special() {
        let result = parse("[ab~Cat(.=1.0)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![1.0, 0.0, 0.0]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_all_implicit() {
        let result = parse("[ab~Cat]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.0, 0.5, 0.5]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_no_implicit() {
        let result = parse("[^ab~Cat(a=0.5,b=0.5)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(true, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.0, 0.5, 0.5]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_const() {
        let result = parse("[ab~Const]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(false, vec!['a', 'b']),
                    }),
                    Some(Dist::Constant(0, 0, 1.0).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_neg_all_implicit() {
        let result = parse("[^ab~Cat]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(true, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![1.0, 0.0, 0.0]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_neg() {
        let result = parse("[^ab~Cat(a=0.1)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(true, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.9, 0.1, 0.0]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_neg_dot() {
        let result = parse("[^ab~Cat(a=0.3,.=0.1)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(true, vec!['a', 'b']),
                    }),
                    Some(Dist::Categorical(vec![0.1, 0.3, 0.6]).index()),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_class_dot_ast() {
        let result = parse("[\\d~Cat(0=0.6,.=0.175)]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(
                            false,
                            vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                        ),
                    }),
                    Some(
                        Dist::Categorical(vec![
                            0.175,
                            0.6,
                            // remaining digits have 0.225 / 9 = 0.025
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                            0.02500000000000001,
                        ])
                        .index(),
                    ),
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_exact_class_named_dist_rest_zero_ast() {
        let result = parse("^[\\d~Cat(1=0.9,.=0.1)]$").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 0,
                kind: Kind::AnchorStart,
            },
            AstNode {
                length: 1,
                kind: Kind::Classified(
                    Box::new(AstNode {
                        length: 1,
                        kind: Kind::Class(
                            false,
                            vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                        ),
                    }),
                    Some(
                        Dist::Categorical(vec![
                            0.1, 0.0, 0.9,
                            // these are zero because 0.1 reserved for other than named params
                            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                        ])
                        .index(),
                    ),
                ),
            },
            AstNode {
                length: 1,
                kind: Kind::AnchorEnd,
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_short_class_ast() {
        let result = parse("\\d").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Class(
                    false,
                    vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_posix_class_ast() {
        let result = parse("[[:digit:]]").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Class(
                    false,
                    vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                ),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_anchor_start_ast() {
        let result = parse("^a").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 0,
                kind: Kind::AnchorStart,
            },
            AstNode {
                length: 1,
                kind: Kind::Literal('a'),
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_anchor_end_ast() {
        let result = parse("a$").unwrap_or_default();
        let expected = vec![
            AstNode {
                length: 1,
                kind: Kind::Literal('a'),
            },
            AstNode {
                length: 1,
                kind: Kind::AnchorEnd,
            },
            AstNode {
                length: 0,
                kind: Kind::Terminal,
            },
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_concat_length() {
        let result = parse("ab").unwrap_or_default().first().unwrap().to_owned();
        assert_eq!(result.length, 2);
    }

    #[test]
    fn test_parser_alternation() {
        assert_eq!(ast_as_str(parse("a|b").unwrap()), "a|b");
        assert_eq!(ast_as_str(parse("a|b|c").unwrap()), "a|b|c");
    }

    #[test]
    fn test_parser_concat() {
        assert_eq!(ast_as_str(parse("abc").unwrap()), "ab.c.");
    }

    #[test]
    fn test_parser_conditional() {
        assert_eq!(ast_as_str(parse("ab?c").unwrap()), "ab?.c.");
    }

    #[test]
    fn test_parser_star() {
        assert_eq!(ast_as_str(parse("ab*c").unwrap()), "ab*.c.");
    }

    #[test]
    fn test_parser_plus() {
        assert_eq!(ast_as_str(parse("ab+c").unwrap()), "ab+.c.");
    }

    #[test]
    fn test_parser_whitespace() {
        assert_eq!(ast_as_str(parse("a c").unwrap()), "a .c.");
        assert_eq!(ast_as_str(parse(" ab").unwrap()), " a.b.");
        assert_eq!(ast_as_str(parse("ab ").unwrap()), "ab. .");
    }

    #[test]
    fn test_parser_parentheses() {
        assert_eq!(ast_as_str(parse("(a)").unwrap()), "a");
        assert_eq!(ast_as_str(parse("(ab)c").unwrap()), "ab.c.");
        assert_eq!(ast_as_str(parse("a(bc)").unwrap()), "abc..");
        assert_eq!(ast_as_str(parse("(a(bc))d").unwrap()), "abc..d.");
        assert_eq!(ast_as_str(parse("(a|b)").unwrap()), "a|b");
        assert_eq!(ast_as_str(parse("(a|b)c").unwrap()), "a|bc."); // TODO not a great representation
    }

    #[test]
    fn test_parser_exact_quantifier() {
        assert_eq!(ast_as_str(parse("a{2}").unwrap()), "a{2}");
        assert_eq!(ast_as_str(parse("a{20}").unwrap()), "a{20}");
    }

    #[test]
    fn test_parser_exact_quantifier_dist() {
        assert_eq!(ast_as_str(parse("a{2~Geo(1.0)}").unwrap()), "a{2~Geo(1)}");
    }

    #[test]
    fn test_parser_exact_class() {
        assert_eq!(ast_as_str(parse("[ab]").unwrap()), "[ab]");
        assert_eq!(ast_as_str(parse("[^ab]").unwrap()), "[^ab]");
    }

    #[test]
    fn test_parser_exact_class_with_dist() {
        assert_eq!(ast_as_str(parse("[ab~Const]").unwrap()), "[[ab]]");
        assert_eq!(ast_as_str(parse("[ab~Geo(1.0)]").unwrap()), "[[ab]~Geo(1)]");
    }
}
