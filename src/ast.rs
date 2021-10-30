use crate::parser::Rule;
use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum AstNode {
    Literal(char),
    Quantifier(char),
    Concatenation,
    Terminal,
}

pub fn build_ast_from_expr(
    mut nodes: Vec<AstNode>,
    pair: pest::iterators::Pair<Rule>,
) -> Vec<AstNode> {
    match pair.as_rule() {
        Rule::Alternation => {
            // TODO
            build_ast_from_expr(nodes, pair.into_inner().next().unwrap())
        }
        Rule::Concat => {
            let mut pair = pair.into_inner();
            let (first, second) = pair.next_tuple().unwrap();
            nodes = build_ast_from_expr(nodes, first);
            nodes = build_ast_from_expr(nodes, second);
            nodes.push(AstNode::Concatenation);
            nodes
        }
        Rule::ConcatMaybe => {
            let mut pair = pair.into_inner();
            let first = pair.next().unwrap();
            nodes = build_ast_from_expr(nodes, first);

            if let Some(val) = pair.next() {
                nodes = build_ast_from_expr(nodes, val);
                nodes.push(AstNode::Concatenation);
            }
            nodes
        }
        Rule::Quantified => {
            let mut pair = pair.into_inner();
            let first = pair.next().unwrap();
            nodes = build_ast_from_expr(nodes, first);
            nodes.push(AstNode::Quantifier(pair.as_str().chars().next().unwrap()));
            nodes
        }
        Rule::Literal => {
            let c = pair.as_str().chars().next().unwrap();
            nodes.push(AstNode::Literal(c));
            nodes
        }
        Rule::EOI => nodes,
        _ => nodes,
    }
}
