use crate::distribution::Dist;
#[allow(dead_code)]
use crate::parser::Rule;
use itertools::Itertools;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct AstNode {
    pub length: usize,
    pub kind: Kind,
    // Keep distirbution out of here
}
// TODO create a helper ::new function to cut boilerplate

#[derive(Debug, PartialEq, Clone)]
pub enum Kind {
    AnchorEnd,
    AnchorStart,
    Alternation(Box<AstNode>, Box<AstNode>),
    Concatenation(Box<AstNode>, Box<AstNode>),
    ExactQuantifier(u64),
    Literal(char),
    Split,
    Start,
    Terminal,
    Quantified(Box<AstNode>, Box<AstNode>, Option<Dist>),
    Quantifier(char),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Kind::Literal(c) => write!(f, "{}", c),
            Kind::Concatenation(l, r) => write!(f, "{}{}.", l, r),
            Kind::Quantified(r, l, Some(q)) => write!(f, "{}{{{}{}}}", l, r, q),
            Kind::Quantified(r, l, None) => match r.kind {
                Kind::Quantifier(_) => write!(f, "{}{}", l, r),
                Kind::ExactQuantifier(_) => write!(f, "{}{{{}}}", l, r),
                _ => unreachable!(),
            },
            Kind::Quantifier(c) => write!(f, "{}", c),
            Kind::ExactQuantifier(n) => write!(f, "{}", n),
            Kind::Alternation(l, r) => write!(f, "{}|{}", l, r),
            Kind::Split => write!(f, "|"),
            Kind::Terminal => write!(f, ""),
            Kind::Start => write!(f, ""),
            Kind::AnchorStart => write!(f, "^"),
            Kind::AnchorEnd => write!(f, "$"),
            // See also fmt::Display for Dist
        }
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)
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
        Rule::AnchorEnd => {
            return AstNode {
                length: 0,
                kind: Kind::AnchorEnd,
            };
        }
        Rule::AnchorStart => {
            return AstNode {
                length: 0,
                kind: Kind::AnchorStart,
            };
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
            // pair.next is ShortQuantifier or LongQuantifier
            let quantifier_ast = build_ast_from_expr(pair.next().unwrap());
            // pair.next is Option<Dist>
            let quantifier_dist = match pair.next() {
                Some(pair) => Some(Dist::complete_from(&quantifier_ast.kind, pair)),
                None => Dist::default_from(&quantifier_ast.kind),
            };
            AstNode {
                length: left_ast.length + quantifier_ast.length,
                kind: Kind::Quantified(
                    Box::new(quantifier_ast),
                    Box::new(left_ast),
                    quantifier_dist,
                ),
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
        Rule::ShortQuantifier => {
            let c = pair.as_str().chars().next().unwrap();
            AstNode {
                length: 1,
                kind: Kind::Quantifier(c),
            }
        }
        Rule::ExactQuantifier => {
            let pair = pair.into_inner().next().unwrap();
            // uses str::parse to convert to appropriate Rust type
            let n: u64 = pair.as_str().parse().unwrap();
            AstNode {
                length: 1,
                kind: Kind::ExactQuantifier(n),
            }
        }
        _ => build_ast_from_expr(pair),
    }
}
