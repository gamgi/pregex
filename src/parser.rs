use crate::ast::{build_ast_from_expr, AstNode, Kind};
use pest::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct RegexParser;

pub fn parse(source: &str) -> std::result::Result<Vec<AstNode>, pest::error::Error<Rule>> {
    let mut ast = Vec::new();
    let pairs = RegexParser::parse(Rule::Regex, source)?;

    for pair in pairs {
        if let Rule::Alternation = pair.as_rule() {
            let node = build_ast_from_expr(pair);
            ast.push(node);
        } else if let Rule::EOI = pair.as_rule() {
            ast.push(AstNode {
                length: 0,
                kind: Kind::Terminal,
            });
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
    fn test_parser_single_length() {
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
    fn test_parser_concat_length() {
        let result = parse("ab").unwrap_or(vec![]).first().unwrap().to_owned();
        assert_eq!(result.length, 2);
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
}
