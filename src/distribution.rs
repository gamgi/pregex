#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::{State, StateParams};
use itertools::Itertools;
use pest;
use statrs::distribution::{Binomial, Discrete, Geometric};
use statrs::statistics::Distribution;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Dist {
    Constant(f64),        // p
    ExactlyTimes(u64),    // n_match
    PGeometric(u64, f64), // n_min, p
    PBinomial(u64, f64),  // n_max, p
}

impl fmt::Display for Dist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dist::Constant(_) => write!(f, ""),
            Dist::ExactlyTimes(n) => write!(f, ""),
            Dist::PGeometric(_, p) => write!(f, "~Geo({})", p),
            Dist::PBinomial(_, p) => write!(f, "~Bin({})", p),
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
            "bin" => {
                let std_dev: f64 = param.as_str().parse().unwrap();
                Dist::PBinomial(n_min, std_dev)
            }
            _ => {
                panic!("Unknown distribution {}", name)
            }
        }
    }
}

/// Evaluate p for state arrows (state.outs) from p0 and number of visits to current node
pub fn evaluate(p0: f64, dist: &Option<Dist>, n: u64) -> (f64, f64) {
    if let Some(dist) = dist {
        return match dist {
            Dist::Constant(p) => (p0 * p, p0 * p),
            Dist::ExactlyTimes(n_match) => {
                if n == *n_match {
                    (0.0, p0)
                } else if n < *n_match {
                    (p0, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
            Dist::PGeometric(n_min, c) => {
                if n < *n_min {
                    return (p0, 0.0);
                }
                let x = n - n_min + 1;
                let p = Geometric::new(*c).unwrap().pmf(x);
                (p * p0, (1.0 - p) * p0)
            }
            Dist::PBinomial(n_max, p) => {
                if n > *n_max {
                    return (p0, 0.0);
                    // return (0.0, 0.0);
                }
                let x = n;
                let p = Binomial::new(*p, *n_max).unwrap().pmf(x);
                ((1.0 - p) * p0, p * p0)
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
        assert_eq!(evaluate(1.0, &Some(Dist::Constant(1.0)), 1), (1.0, 1.0));
        assert_eq!(evaluate(0.5, &Some(Dist::Constant(1.0)), 1), (0.5, 0.5));
        assert_eq!(evaluate(1.0, &Some(Dist::Constant(0.5)), 1), (0.5, 0.5));
    }

    #[test]
    fn test_distribution_exactly_times() {
        assert_eq!(evaluate(1.0, &Some(Dist::ExactlyTimes(2)), 0), (1.0, 0.0));
        assert_eq!(evaluate(1.0, &Some(Dist::ExactlyTimes(2)), 1), (1.0, 0.0));
        assert_eq!(evaluate(1.0, &Some(Dist::ExactlyTimes(2)), 2), (0.0, 1.0));
        assert_eq!(evaluate(1.0, &Some(Dist::ExactlyTimes(2)), 3), (0.0, 0.0));

        assert_eq!(evaluate(0.5, &Some(Dist::ExactlyTimes(2)), 0), (0.5, 0.0));
        assert_eq!(evaluate(0.5, &Some(Dist::ExactlyTimes(2)), 1), (0.5, 0.0));
        assert_eq!(evaluate(0.5, &Some(Dist::ExactlyTimes(2)), 2), (0.0, 0.5));
        assert_eq!(evaluate(0.5, &Some(Dist::ExactlyTimes(2)), 3), (0.0, 0.0));
    }

    #[test]
    fn test_distribution_geometric_1_or_more() {
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(1, 0.5)), 0),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(1, 0.5)), 1),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(1, 0.5)), 2),
            (0.25, 0.75)
        );

        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(1, 0.5)), 0),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(1, 0.5)), 1),
            (0.25, 0.25)
        );
        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(1, 0.5)), 2),
            (0.125, 0.375)
        );
    }

    #[test]
    fn test_distribution_geometric_2_or_more() {
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(2, 0.5)), 0),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(2, 0.5)), 1),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, &Some(Dist::PGeometric(2, 0.5)), 2),
            (0.5, 0.5)
        );

        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(2, 0.5)), 0),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(2, 0.5)), 1),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, &Some(Dist::PGeometric(2, 0.5)), 2),
            (0.25, 0.25)
        );
    }
    #[test]
    fn test_distribution_binomial_degenerate() {
        // If p = 1 the distribution is concentrated at n
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(0, 1.0)), 0), (0.0, 1.0));
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(0, 1.0)), 1), (1.0, 0.0));
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(1, 1.0)), 1), (0.0, 1.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_1() {
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(1, 0.5)), 0), (0.5, 0.5));
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(1, 0.5)), 1), (0.5, 0.5));
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(1, 0.5)), 2), (1.0, 0.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_2() {
        assert_eq!(
            evaluate(1.0, &Some(Dist::PBinomial(2, 0.5)), 0),
            (0.75, 0.25)
        );
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(2, 0.5)), 1), (0.5, 0.5));
        assert_eq!(
            evaluate(1.0, &Some(Dist::PBinomial(2, 0.5)), 2),
            (0.75, 0.25)
        );
        assert_eq!(evaluate(1.0, &Some(Dist::PBinomial(2, 0.5)), 3), (1.0, 0.0));
    }
}
