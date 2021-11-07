use crate::ast::{build_ast_from_expr, AstNode, Kind};
use pest::Parser;

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

    fn ast_as_str(asts: Vec<AstNode>) -> String {
        asts.into_iter()
            .map(|ast| ast.to_string())
            .collect::<Vec<String>>()
            .join("")
    }

    #[test]
    fn test_parser_single_ast() {
        let result = parse("a").unwrap_or(vec![]);
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
        let result = parse("a|b").unwrap_or(vec![]);
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
        let result = parse("a?").unwrap_or(vec![]);
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
        let result = parse("a?b").unwrap_or(vec![]);
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
    fn test_parser_concat_length() {
        let result = parse("ab").unwrap_or(vec![]).first().unwrap().to_owned();
        assert_eq!(result.length, 2);
    }

    #[test]
    fn test_parser_alternation() {
        assert_eq!(ast_as_str(parse("a|b").unwrap()), "a|b$");
        assert_eq!(ast_as_str(parse("a|b|c").unwrap()), "a|b|c$");
    }

    #[test]
    fn test_parser_concat() {
        assert_eq!(ast_as_str(parse("abc").unwrap()), "ab.c.$");
    }

    #[test]
    fn test_parser_conditional() {
        assert_eq!(ast_as_str(parse("ab?c").unwrap()), "ab?.c.$");
    }

    #[test]
    fn test_parser_star() {
        assert_eq!(ast_as_str(parse("ab*c").unwrap()), "ab*.c.$");
    }

    #[test]
    fn test_parser_plus() {
        assert_eq!(ast_as_str(parse("ab+c").unwrap()), "ab+.c.$");
    }

    #[test]
    fn test_parser_whitespace() {
        assert_eq!(ast_as_str(parse("a c").unwrap()), "a .c.$");
        assert_eq!(ast_as_str(parse(" ab").unwrap()), " a.b.$");
        assert_eq!(ast_as_str(parse("ab ").unwrap()), "ab. .$");
    }

    #[test]
    fn test_parser_parentheses() {
        assert_eq!(ast_as_str(parse("(a)").unwrap()), "a$");
        assert_eq!(ast_as_str(parse("(ab)c").unwrap()), "ab.c.$");
        assert_eq!(ast_as_str(parse("a(bc)").unwrap()), "abc..$");
        assert_eq!(ast_as_str(parse("(a(bc))d").unwrap()), "abc..d.$");
        assert_eq!(ast_as_str(parse("(a|b)").unwrap()), "a|b$");
        assert_eq!(ast_as_str(parse("(a|b)c").unwrap()), "a|bc.$"); // TODO not a great representation
    }
}
