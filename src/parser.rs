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
            let mut nodes = build_ast_from_expr(Vec::new(), pair);
            ast.append(&mut nodes);
        }
    }
    ast.push(AstNode::Terminal);
    Ok(ast)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser_concat() {
        let result = parse("ab").unwrap_or(vec![]);
        let expected = vec![
            AstNode::Literal('a'),
            AstNode::Literal('b'),
            AstNode::Terminal,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parser_quantifier() {
        let result = parse("ab?c").unwrap_or(vec![]);
        let expected = vec![
            AstNode::Literal('a'),
            AstNode::Literal('b'),
            AstNode::Quantifier('?'),
            AstNode::Literal('c'),
            AstNode::Terminal,
        ];
        assert_eq!(result, expected);
    }
}
