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
    Constant(f64),        // p
    ExactlyTimes(u64),    // n_min
    PGeometric(u64, f64), // n_min, p
}

impl fmt::Display for Dist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dist::Constant(_) => write!(f, ""),
            Dist::ExactlyTimes(n) => write!(f, ""),
            Dist::PGeometric(_, p) => write!(f, "~Geo({})", p),
        }
    }
}

impl Dist {
    pub fn default_from(quantifier_kind: &Kind) -> Option<Self> {
        match quantifier_kind {
            Kind::ExactQuantifier(n) => Some(Dist::ExactlyTimes(*n)),
            _ => None,
        }
    }

    /// Distribution from quantifier kind and distribution params
    ///
    /// Eg. complete_from(ExactQuantifier(2), Dist::Normal(sigma))
    /// would return a Normal distribution centered at 2.
    pub fn complete_from(
        quantifier_kind: &Kind,
        quantifier_dist_pair: pest::iterators::Pair<'_, crate::parser::Rule>,
    ) -> Self {
        let n_min = match quantifier_kind {
            Kind::ExactQuantifier(n) => *n,
            _ => 1, // TODO what to do here
        };

        let (name, param) = quantifier_dist_pair.into_inner().collect_tuple().unwrap();
        let name = name.as_span().as_str().to_lowercase();
        match name.as_str() {
            "geo" => {
                let p: f64 = param.as_str().parse().unwrap();
                Dist::PGeometric(n_min, p)
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
            Dist::PGeometric(match_n, c) => {
                if *n < *match_n {
                    return (p0, 0.0);
                }
                let x = *n - *match_n + 1;
                let p = Geometric::new(*c).unwrap().pmf(x);
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
    fn test_distribution_geometric_1_or_more() {
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 0)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 1)),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 2)),
            (0.25, 0.75)
        );

        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 0)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 1)),
            (0.25, 0.25)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(1, 0.5)), 0.0, 2)),
            (0.125, 0.375)
        );
    }

    #[test]
    fn test_distribution_geometric_2_or_more() {
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 0)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 1)),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 2)),
            (0.5, 0.5)
        );

        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 0)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 1)),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &(Some(Dist::PGeometric(2, 0.5)), 0.0, 2)),
            (0.25, 0.25)
        );
    }
}
