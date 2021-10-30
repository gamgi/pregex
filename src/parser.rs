use crate::ast::{build_ast_from_expr, AstNode};
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
            ast.push(AstNode::Terminal);
        }
    }
    Ok(ast)
}

#[cfg(test)]
mod test {
    use super::*;

    fn ast_as_str(asts: Vec<AstNode>) -> String {
        asts.into_iter()
            .map(|mut ast| ast.as_str())
            .collect::<Vec<String>>()
            .join("")
    }

    #[test]
    fn test_parser_single() {
        let result = parse("a").unwrap_or(vec![]);
        let expected = vec![AstNode::Literal('a'), AstNode::Terminal];
        assert_eq!(result, expected);
    }
    #[test]
    fn test_parser_concat() {
        let result = ast_as_str(parse("abc").unwrap());
        let expected = "ab.c.$".to_string();
        assert_eq!(result, expected);
    }
    #[test]
    fn test_parser_quantifier() {
        let result = ast_as_str(parse("ab?c").unwrap());
        let expected = "ab?.c.$".to_string();
        assert_eq!(result, expected);
    }
}
