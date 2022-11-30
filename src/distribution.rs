#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::{State, StateParams};
use itertools::Itertools;

use pest::iterators::Pair;
use statrs::distribution::{Bernoulli, Binomial, Discrete, Geometric};
use statrs::statistics::Distribution;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Dist {
    Constant(f64),        // p
    ExactlyTimes(u64),    // n_match
    PGeometric(u64, f64), // n_min, p
    PBinomial(u64, f64),  // n_max, p
    PBernoulli(u64, f64), // n_max, p
}

impl fmt::Display for Dist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dist::Constant(_) => write!(f, ""),
            Dist::ExactlyTimes(n) => write!(f, ""),
            Dist::PGeometric(_, p) => write!(f, "~Geo({})", p),
            Dist::PBinomial(_, p) => write!(f, "~Bin({})", p),
            Dist::PBernoulli(_, p) => write!(f, "~Ber({})", p),
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
        quantifier_dist_pair: Pair<'_, crate::parser::Rule>,
    ) -> Self {
        let n = match quantifier_kind {
            Kind::ExactQuantifier(n) => *n,
            _ => 1, // TODO what to do here
        };

        let (name, param) = quantifier_dist_pair.into_inner().collect_tuple().unwrap();
        let name = name.as_span().as_str().to_lowercase();
        match name.as_str() {
            "geo" => {
                let p: f64 = param.as_str().parse().unwrap();
                Dist::PGeometric(n, p)
            }
            "bin" => {
                let p: f64 = param.as_str().parse().unwrap();
                Dist::PBinomial(n, p)
            }
            "ber" => {
                let p: f64 = param.as_str().parse().unwrap();
                Dist::PBernoulli(n, p)
            }
            _ => {
                panic!("Unknown distribution {}", name)
            }
        }
    }

    /// Evaluate (p0, p1) for state arrows (state.outs)
    pub fn evaluate(&self, n: u64, log: bool) -> (f64, f64) {
        // Special distributions
        match self {
            Dist::Constant(p) => match log {
                true => return (*p, *p),
                false => return (*p, *p),
            },
            Dist::ExactlyTimes(n_match) => {
                // does not depend on log
                if n == *n_match {
                    return (0.0, 1.0);
                } else if n < *n_match {
                    return (1.0, 0.0);
                } else {
                    return (0.0, 0.0);
                }
            }
            _ => {}
        };

        // Evaluate point mass function from distribution
        let p = match self {
            Dist::PGeometric(n_min, c) => {
                if n < *n_min {
                    return (1.0, 0.0);
                }
                let x = n - n_min + 1;
                match log {
                    true => Geometric::new(*c).unwrap().ln_pmf(x),
                    false => Geometric::new(*c).unwrap().pmf(x),
                }
            }
            Dist::PBinomial(n_max, p) => {
                if n > *n_max {
                    return (0.0, 0.0);
                }
                let x = n;
                match log {
                    true => Binomial::new(*p, *n_max).unwrap().ln_pmf(x),
                    false => Binomial::new(*p, *n_max).unwrap().pmf(x),
                }
            }
            Dist::PBernoulli(n_max, p) => {
                if n > *n_max {
                    return (1.0, 0.0);
                }
                let x = n;
                match log {
                    true => Bernoulli::new(*p).unwrap().ln_pmf(x),
                    false => Bernoulli::new(*p).unwrap().pmf(x),
                }
            }
            _ => unreachable!(),
        };

        // Calculate complement and return as out arrow probabilities (p0, p1)
        match log {
            true => ((1. - p.exp()).ln(), p),
            false => (1. - p, p),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_distribution_constant() {
        assert_eq!(Dist::Constant(1.0).evaluate(1, false), (1.0, 1.0));
        assert_eq!(Dist::Constant(0.5).evaluate(1, false), (0.5, 0.5));
    }

    #[test]
    fn test_distribution_exactly_times() {
        assert_eq!(Dist::ExactlyTimes(2).evaluate(0, false), (1.0, 0.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluate(1, false), (1.0, 0.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluate(2, false), (0.0, 1.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluate(3, false), (0.0, 0.0));
    }

    #[test]
    #[rustfmt::skip]
    fn test_distribution_geometric_1_or_more() {
        assert_eq!(Dist::PGeometric(1, 0.5).evaluate( 0, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(1, 0.5).evaluate( 1, false), (0.5, 0.5));
        assert_eq!(Dist::PGeometric(1, 0.5).evaluate( 2, false), (0.75, 0.25));
    }

    #[test]
    fn test_distribution_geometric_2_or_more() {
        assert_eq!(Dist::PGeometric(2, 0.5).evaluate(0, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(2, 0.5).evaluate(1, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(2, 0.5).evaluate(2, false), (0.5, 0.5));
    }

    #[test]
    fn test_distribution_binomial_degenerate() {
        // p = 0, the distribution is concentrated at 0
        assert_eq!(Dist::PBinomial(0, 1.0).evaluate(0, false), (0.0, 1.0));
        assert_eq!(Dist::PBinomial(0, 1.0).evaluate(1, false), (0.0, 0.0));
        assert_eq!(Dist::PBinomial(0, 1.0).evaluate(2, false), (0.0, 0.0));

        // p = 1, the distribution is concentrated at n
        assert_eq!(Dist::PBinomial(1, 1.0).evaluate(1, false), (0.0, 1.0));
        assert_eq!(Dist::PBinomial(1, 1.0).evaluate(2, false), (0.0, 0.0));
        assert_eq!(Dist::PBinomial(5, 1.0).evaluate(5, false), (0.0, 1.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_1() {
        assert_eq!(Dist::PBinomial(1, 0.5).evaluate(0, false), (0.5, 0.5));
        assert_eq!(Dist::PBinomial(1, 0.5).evaluate(1, false), (0.5, 0.5));
        assert_eq!(Dist::PBinomial(1, 0.5).evaluate(2, false), (0.0, 0.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_2() {
        use Dist::PBinomial;
        assert_eq!(PBinomial(2, 0.5).evaluate(0, false), (0.75, 0.25));
        assert_eq!(PBinomial(2, 0.5).evaluate(1, false), (0.5, 0.5));
        assert_eq!(PBinomial(2, 0.5).evaluate(2, false), (0.75, 0.25));
        assert_eq!(PBinomial(2, 0.5).evaluate(3, false), (0.0, 0.0));
    }

    #[test]
    fn test_distribution_bernoulli() {
        assert_eq!(Dist::PBernoulli(1, 0.5).evaluate(0, false), (0.5, 0.5));
        assert_eq!(Dist::PBernoulli(1, 0.5).evaluate(1, false), (0.5, 0.5));
        assert_eq!(Dist::PBernoulli(2, 0.5).evaluate(2, false), (1.0, 0.0));
    }
}
