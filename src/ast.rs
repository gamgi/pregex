use crate::parser::Rule;
use itertools::Itertools;

#[derive(Debug, PartialEq, Clone)]
pub enum AstNode {
    Literal(char),
    Quantifier(char, Box<AstNode>),
    Concatenation(Box<AstNode>, Box<AstNode>),
    Terminal,
}

impl AstNode {
    pub fn as_str(&mut self) -> String {
        match self {
            AstNode::Literal(c) => format!("{}", c),
            AstNode::Concatenation(l, r) => format!("{}{}.", l.as_str(), r.as_str()),
            AstNode::Quantifier(c, r) => format!("{}{}", r.as_str(), c),
            AstNode::Terminal => "$".to_string(),
        }
    }
}

pub fn build_ast_from_expr(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::Alternation => {
            // TODO
            build_ast_from_expr(pair.into_inner().next().unwrap())
        }
        Rule::Concat => {
            let mut pair = pair.into_inner();
            let (left, right) = pair.next_tuple().unwrap();
            AstNode::Concatenation(
                Box::new(build_ast_from_expr(left)),
                Box::new(build_ast_from_expr(right)),
            )
        }
        Rule::ConcatMaybe => {
            let mut pair = pair.into_inner();
            let left = pair.next().unwrap();

            if let Some(right) = pair.next() {
                return AstNode::Concatenation(
                    Box::new(build_ast_from_expr(left)),
                    Box::new(build_ast_from_expr(right)),
                );
            }
            build_ast_from_expr(left)
        }
        Rule::Quantified => {
            let mut pair = pair.into_inner();
            let left = pair.next().unwrap();
            let c = pair.as_str().chars().next().unwrap();
            AstNode::Quantifier(c, Box::new(build_ast_from_expr(left)))
        }
        Rule::Literal => {
            let c = pair.as_str().chars().next().unwrap();
            AstNode::Literal(c)
        }
        Rule::EOI => AstNode::Terminal,
        _ => build_ast_from_expr(pair),
    }
}
