#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Dist {
    Constant(f32),
    ExactlyTimes(u32),
}

pub type StateParams = (Dist, f32, u32); // (c, p, visits)

pub fn evaluate(p: f32, params: Option<&StateParams>) -> (f32, f32) {
    if let Some((dist, _, n)) = params {
        return match dist {
            Dist::Constant(c) => (p * c, p * c),
            Dist::ExactlyTimes(match_n) => {
                if n == match_n {
                    (0.0, p)
                } else if n < match_n {
                    (p, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
        };
    }
    (p, p)
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
}
