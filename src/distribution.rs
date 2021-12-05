#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use itertools::Itertools;
use pest;
use statrs::distribution::{Discrete, Geometric};
use statrs::statistics::Distribution;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Dist {
    Constant(f64),
    ExactlyTimes(u64),
    PGeometric(f64),
}

impl fmt::Display for Dist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dist::Constant(_) => write!(f, ""),
            Dist::ExactlyTimes(n) => write!(f, ""),
            Dist::PGeometric(p) => write!(f, "~Geo({})", p),
        }
    }
}

impl From<pest::iterators::Pair<'_, crate::parser::Rule>> for Dist {
    fn from(quantifier_dist_pair: pest::iterators::Pair<'_, crate::parser::Rule>) -> Self {
        let (name, param) = quantifier_dist_pair.into_inner().collect_tuple().unwrap();
        let name = name.as_span().as_str().to_lowercase();
        match name.as_str() {
            "geo" => {
                let p: f64 = param.as_str().parse().unwrap();
                Dist::PGeometric(p)
            }
            _ => {
                panic!("Unknown distribution {}", name)
            }
        }
    }
}

impl Dist {
    pub fn default_from(quantifier_kind: &Kind) -> Option<Dist> {
        match quantifier_kind {
            Kind::ExactQuantifier(n) => Some(Dist::ExactlyTimes(*n)),
            _ => None,
        }
    }
}

pub type StateParams = (Option<Dist>, f64, u64); // (distribution, p, visits)

/// Evaluate p for state arrows (state.outs) from base p (p0) and
/// state's distribution parameters.
pub fn evaluate(p0: f64, params: &StateParams) -> (f64, f64) {
    if let (Some(dist), _, n) = params {
        return match dist {
            Dist::Constant(p) => (p0 * p, p0 * p),
            Dist::ExactlyTimes(match_n) => {
                if n == match_n {
                    (0.0, p0)
                } else if n < match_n {
                    (p0, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
            Dist::PGeometric(c) => {
                if *n == 0 {
                    return (p0, 0.0);
                }
                let p = Geometric::new(*c).unwrap().pmf(*n);
                (p * p0, (1.0 - p) * p0)
            }
        };
    }
    (p0, p0)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_distribution_constant() {
        assert_eq!(
            evaluate(1.0, &(Some(Dist::Constant(1.0)), 0.0, 1)),
            (1.0, 1.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::Constant(1.0)), 0.0, 1)),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::Constant(0.5)), 0.0, 1)),
            (0.5, 0.5)
        );
    }

    #[test]
    fn test_distribution_exactly_times() {
        assert_eq!(
            evaluate(1.0, &(Some(Dist::ExactlyTimes(2)), 0.0, 0)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::ExactlyTimes(2)), 0.0, 1)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::ExactlyTimes(2)), 0.0, 2)),
            (0.0, 1.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::ExactlyTimes(2)), 0.0, 3)),
            (0.0, 0.0)
        );

        assert_eq!(
            evaluate(0.5, &(Some(Dist::ExactlyTimes(2)), 0.0, 0)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::ExactlyTimes(2)), 0.0, 1)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::ExactlyTimes(2)), 0.0, 2)),
            (0.0, 0.5)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::ExactlyTimes(2)), 0.0, 3)),
            (0.0, 0.0)
        );
    }

    #[test]
    fn test_distribution_geometric() {
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(0.5)), 0.0, 0)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(0.5)), 0.0, 1)),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(0.5)), 0.0, 2)),
            (0.25, 0.75)
        );

        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(0.5)), 0.0, 0)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(0.5)), 0.0, 1)),
            (0.25, 0.25)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(0.5)), 0.0, 2)),
            (0.125, 0.375)
        );
    }
}
