#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use itertools::Itertools;
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

pub type StateParams = (Dist, f64, u64); // (distribution, p, visits)

pub fn evaluate(p0: f64, params: Option<&StateParams>) -> (f64, f64) {
    if let Some((dist, _, n)) = params {
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
            evaluate(1.0, Some(&(Dist::Constant(1.0), 0.0, 1))),
            (1.0, 1.0)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::Constant(1.0), 0.0, 1))),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::Constant(0.5), 0.0, 1))),
            (0.5, 0.5)
        );
    }

    #[test]
    fn test_distribution_exactly_times() {
        assert_eq!(
            evaluate(1.0, Some(&(Dist::ExactlyTimes(2), 0.0, 0))),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::ExactlyTimes(2), 0.0, 1))),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::ExactlyTimes(2), 0.0, 2))),
            (0.0, 1.0)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::ExactlyTimes(2), 0.0, 3))),
            (0.0, 0.0)
        );

        assert_eq!(
            evaluate(0.5, Some(&(Dist::ExactlyTimes(2), 0.0, 0))),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::ExactlyTimes(2), 0.0, 1))),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::ExactlyTimes(2), 0.0, 2))),
            (0.0, 0.5)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::ExactlyTimes(2), 0.0, 3))),
            (0.0, 0.0)
        );
    }

    #[test]
    fn test_distribution_geometric() {
        assert_eq!(
            evaluate(1.0, Some(&(Dist::PGeometric(0.5), 0.0, 0))),
            (1.0, 0.0)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::PGeometric(0.5), 0.0, 1))),
            (0.5, 0.5)
        );
        assert_eq!(
            evaluate(1.0, Some(&(Dist::PGeometric(0.5), 0.0, 2))),
            (0.25, 0.75)
        );

        assert_eq!(
            evaluate(0.5, Some(&(Dist::PGeometric(0.5), 0.0, 0))),
            (0.5, 0.0)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::PGeometric(0.5), 0.0, 1))),
            (0.25, 0.25)
        );
        assert_eq!(
            evaluate(0.5, Some(&(Dist::PGeometric(0.5), 0.0, 2))),
            (0.125, 0.375)
        );
    }
}
