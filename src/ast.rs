#[allow(dead_code)]
use crate::parser::Rule;
use itertools::Itertools;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct AstNode {
    pub length: usize,
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Kind {
    Literal(char),
    Quantified(Box<AstNode>, Box<AstNode>),
    Quantifier(char),
    Concatenation(Box<AstNode>, Box<AstNode>),
    Alternation(Box<AstNode>, Box<AstNode>),
    Split,
    Terminal,
    Start,
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            Kind::Literal(c) => write!(f, "{}", c),
            Kind::Concatenation(l, r) => write!(f, "{}{}.", l, r),
            Kind::Quantified(r, l) => write!(f, "{}{}", l, r),
            Kind::Quantifier(c) => write!(f, "{}", c),
            Kind::Alternation(l, r) => write!(f, "{}|{}", l, r),
            Kind::Split => write!(f, "|"),
            Kind::Terminal => write!(f, "$"),
            Kind::Start => write!(f, "^"),
        }
    }
}

pub fn build_ast_from_expr(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::Alternation => {
            let mut pair = pair.into_inner();
            let left = pair.next().unwrap();
            let left_ast = build_ast_from_expr(left);

            if let Some(right) = pair.next() {
                let right_ast = build_ast_from_expr(right);
                return AstNode {
                    length: left_ast.length + right_ast.length + 1,
                    kind: Kind::Alternation(Box::new(left_ast), Box::new(right_ast)),
                };
            }
            left_ast
        }
        Rule::Concat | Rule::Concats => {
            let mut pair = pair.into_inner();
            let (left, right) = pair.next_tuple().unwrap();
            let left_ast = build_ast_from_expr(left);
            let right_ast = build_ast_from_expr(right);
            AstNode {
                length: left_ast.length + right_ast.length,
                kind: Kind::Concatenation(Box::new(left_ast), Box::new(right_ast)),
            }
        }
        Rule::Quantified => {
            let mut pair = pair.into_inner();
            let left_ast = build_ast_from_expr(pair.next().unwrap());
            let c = pair.as_str().chars().next().unwrap(); // HACK
            let quantifier_ast = AstNode {
                length: 1,
                kind: Kind::Quantifier(c),
            };
            AstNode {
                length: left_ast.length + quantifier_ast.length,
                kind: Kind::Quantified(Box::new(quantifier_ast), Box::new(left_ast)),
            }
        }
        Rule::Literal => {
            let c = pair.as_str().chars().next().unwrap();
            AstNode {
                length: 1,
                kind: Kind::Literal(c),
            }
        }
        Rule::EOI => AstNode {
            length: 0,
            kind: Kind::Terminal,
        },
        _ => build_ast_from_expr(pair),
    }
}
