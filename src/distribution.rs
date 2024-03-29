#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use crate::parser::Rule;
use crate::regex_state::Token;
use itertools::Itertools;

use pest::iterators::Pair;
use statrs::distribution::{Bernoulli, Binomial, Categorical, Discrete, Geometric};
use statrs::statistics::Distribution;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum Dist {
    Categorical(Vec<f64>),     // p[]
    Constant(u64, u64, f64),   // n_min, n_max, p
    ExactlyTimes(u64),         // n_match
    PGeometric(u64, u64, f64), // n_min, n_max, p
    PBinomial(u64, u64, f64),  // n_min, n_max, p
    PBernoulli(u64, u64, f64), // n_min, n_max, p
    PZipf(u64, u64, f64),      // n_min, n_max, s
}

impl fmt::Display for Dist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dist::Categorical(_) => write!(f, "~Cat"),
            Dist::Constant(_, _, _) => write!(f, ""),
            Dist::ExactlyTimes(_) => write!(f, ""),
            Dist::PGeometric(_, _, p) => write!(f, "~Geo({})", p),
            Dist::PBinomial(_, _, p) => write!(f, "~Bin({})", p),
            Dist::PBernoulli(_, _, p) => write!(f, "~Ber({})", p),
            Dist::PZipf(_, _, p) => write!(f, "~Zipf({})", p),
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

    /// Distribution from kind and distribution params
    ///
    /// Eg. complete_from(ExactQuantifier(2), Dist::Normal(sigma))
    /// would return a Normal distribution centered at 2.
    pub fn complete_from(kind: &Kind, dist_pair: Pair<'_, crate::parser::Rule>) -> Self {
        let n = match kind {
            Kind::ExactQuantifier(n) => *n,
            _ => 0, // required n is zero
        };
        let (is_negate, c) = match kind {
            Kind::Class(neg, c) => (*neg, Some(c)),
            _ => (false, None),
        };

        let mut pair = dist_pair.into_inner();
        let name = pair.next().unwrap().as_span().as_str().to_lowercase();

        // Parse parameters, that may be supplied in various formats
        let (params, params_named): (Vec<Option<&str>>, Vec<_>) = pair
            .map(|p| match p.as_rule() {
                // indexed form, i.e. (0.0, 0.1, 0.2, ...)
                Rule::IndexParam => (Some(p.as_str()), None),
                // named form, i.e. (a=0.0, b=0.1, c=0.2, ...)
                Rule::NamedParam => {
                    let p_str = p.as_str();
                    let (key, val) = p_str.trim().split_at(p_str.find('=').unwrap());
                    (None, Some((key, &val[1..])))
                }
                _ => unreachable!(),
            })
            .unzip();
        let params: Vec<&str> = params.into_iter().flatten().collect();
        let params_named: HashMap<&str, &str> = params_named.into_iter().flatten().collect();

        // Instantiate distribution with possible default parameters
        match name.as_str() {
            "const" => {
                let p: f64 = params.first().unwrap_or(&"1.0").parse().unwrap();
                Dist::Constant(n, n, p)
            }
            "geo" => {
                let p: f64 = params.first().unwrap_or(&"0.5").parse().unwrap();
                Dist::PGeometric(n, u64::MAX, p)
            }
            "ber" => {
                let p: f64 = params.first().unwrap_or(&"1.0").parse().unwrap();
                Dist::PBernoulli(0, 2, p)
            }
            "bin" => {
                let p = params.first().unwrap_or(&"1.0").parse::<f64>().unwrap();
                let n = match c {
                    Some(c) => match c.len() {
                        0 => 0,
                        // binomial distribution has support for x >= 0
                        n => (n - 1) as u64,
                    },
                    None => n,
                };
                Dist::PBinomial(0, n, p)
            }
            "cat" => {
                let params_named: HashMap<char, f64> = params_named
                    .into_iter()
                    .map(|(k, v)| (k.chars().next().unwrap(), v.parse().unwrap()))
                    .collect();
                let n_explicit = params_named.iter().filter(|&(k, _)| *k != '.').count();
                let n_implicit = match c {
                    Some(chars) => usize::max(1, chars.len() - n_explicit),
                    None => 1,
                };

                let prob_mass = match is_negate {
                    true => {
                        // Calculate excplicit and remainder mass
                        let explicit_mass = params_named
                            .iter()
                            .filter_map(|(k, v)| match *k == '.' {
                                true => None,
                                false => Some(*v),
                            })
                            .sum::<f64>();
                        let remainder_mass = match params_named.get(&'.') {
                            Some(v) => *v,
                            None => f64::max(0.0, 1.0 - explicit_mass),
                        };

                        // Rest of proability mass is implicit
                        let implicit_mass = f64::max(0.0, 1.0 - explicit_mass - remainder_mass);

                        // Probability mass for valid character without given weight
                        let p_implicit = f64::max(0.0, implicit_mass) / n_implicit as f64;

                        let mut prob_mass: Vec<f64> = c
                            .expect("chars to be passed")
                            .iter()
                            .map(|c| *params_named.get(c).unwrap_or(&p_implicit))
                            .collect();

                        // Insert remainder as first item to ensure prob_mass sum is not zero
                        prob_mass.insert(0, remainder_mass);
                        prob_mass
                    }
                    false => {
                        // Calculate excplicit and remainder mass
                        let explicit_mass = params_named.values().sum::<f64>();
                        let implicit_mass = f64::max(0.0, 1.0 - explicit_mass);

                        // Probability mass for valid character without given weight
                        let p_implicit = f64::max(0.0, implicit_mass) / n_implicit as f64;
                        let mut prob_mass: Vec<f64> = c
                            .expect("chars to be passed")
                            .iter()
                            .map(|c| *params_named.get(c).unwrap_or(&p_implicit))
                            .collect();

                        // Probability mass for invalid character
                        let p_remainder = match params_named.get(&'.') {
                            Some(v) => *v,
                            None => f64::max(0.0, 1.0 - prob_mass.iter().sum::<f64>()),
                        };

                        // Insert remainder as first item to ensure prob_mass sum is not zero
                        prob_mass.insert(0, p_remainder);
                        prob_mass
                    }
                };
                Dist::Categorical(prob_mass)
            }
            "zipf" => {
                let p: f64 = params.first().unwrap_or(&"1.0").parse().unwrap();
                let n = match c {
                    Some(c) => c.len() as u64,
                    None => n,
                };
                Dist::PZipf(0, n, p)
            }
            _ => {
                panic!("Unknown distribution {}", name)
            }
        }
    }

    /// Test helper
    pub(crate) fn evaluated(&self, x: u64, log: bool) -> (f64, f64) {
        self.evaluate(x, log)
    }

    /// Evaluate (p0, p1) for state arrows (state.outs)
    pub fn evaluate(&self, x: u64, log: bool) -> (f64, f64) {
        // Special distributions
        match self {
            Dist::Constant(n_min, n_max, p) => {
                let n = x;
                return match n >= *n_min && n <= *n_max {
                    true => match log {
                        true => return (p.ln(), p.ln()),
                        false => return (*p, *p),
                    },
                    false => (0.0, 0.0),
                };
            }
            #[allow(clippy::comparison_chain)]
            Dist::ExactlyTimes(n_match) => {
                let n = x;
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
            Dist::PGeometric(n_min, _, c) => {
                if x < *n_min {
                    return (1.0, 0.0);
                }
                let x = x - n_min + 1;
                match log {
                    true => Geometric::new(*c).unwrap().ln_pmf(x),
                    false => Geometric::new(*c).unwrap().pmf(x),
                }
            }
            Dist::PBinomial(_, n_max, p) => {
                if x > *n_max {
                    return (0.0, 0.0);
                }
                match log {
                    true => Binomial::new(*p, *n_max).unwrap().ln_pmf(x),
                    false => Binomial::new(*p, *n_max).unwrap().pmf(x),
                }
            }
            Dist::PBernoulli(_, n_max, p) => {
                if x > *n_max {
                    return (1.0, 0.0);
                }
                match log {
                    true => Bernoulli::new(*p).unwrap().ln_pmf(x),
                    false => Bernoulli::new(*p).unwrap().pmf(x),
                }
            }
            Dist::PZipf(_, n_max, s) => {
                let p = zipf(x, *s, *n_max);
                return (1. - p, p);
            }
            Dist::Categorical(prob_mass) => {
                let p = match log {
                    true => Categorical::new(prob_mass).unwrap().ln_pmf(x),
                    false => Categorical::new(prob_mass).unwrap().pmf(x),
                };
                return (1. - p, p);
            }
            _ => unreachable!(),
        };

        // Calculate complement and return as out arrow probabilities (p0, p1)
        match log {
            true => ((1. - p.exp()).ln(), p),
            false => (1. - p, p),
        }
    }

    pub fn count(self) -> DistLink {
        DistLink::Counted(self)
    }

    pub fn index(self) -> DistLink {
        DistLink::Indexed(self)
    }
}

/// Calculates the probability mass function for the zipf distribution at `x`
// The Zipf distribution reduces to the Zeta distribution as n -> inf
fn zipf(x: u64, a: f64, n: u64) -> f64 {
    // Support zero for consistency
    if x == 0 {
        return 0.0;
    }

    let normalizer = generalized_harmonic_number(n, a);
    (1.0 / (x as f64).powf(a)) / normalizer
}

fn generalized_harmonic_number(n: u64, m: f64) -> f64 {
    (1..(n + 1)).map(|n_i| 1.0 / (n_i as f64).powf(m)).sum()
}

/// Link for mapping state parameters to distribution parameters
#[derive(Debug, PartialEq, Clone)]
pub enum DistLink {
    /// Distribution indexed by number of visits
    Counted(Dist),
    /// Distribution indexed by token position
    Indexed(Dist),
}

impl DistLink {
    /// Calculates the probability mass function for the linked distribution.
    ///
    /// Equivalent to pmf(link(token, n_visits, ...))
    pub fn pmf_link(
        &self,
        token: &Token,
        x: Option<u64>,
        kind: &Kind,
        is_inverse: bool,
        log: bool,
    ) -> (f64, f64) {
        let (p0, p1) = match self {
            DistLink::Counted(d) => d.evaluate(x.unwrap_or(0), log),
            DistLink::Indexed(d) => {
                let c = match token {
                    Kind::Literal(c) => c,
                    _ => {
                        // skip non literals for now
                        return (0., 0.);
                    }
                };

                // TODO add to PR comments that this changed, so no longer pass Option<x> instead handle None in pmf_link

                if let Some(x) = x {
                    match d {
                        // zipf distribution has support for x > 0
                        Dist::PZipf(_, _, _) => d.evaluate(x + 1, log),
                        // categorical has support for x > 0 due to p_rest
                        Dist::Categorical(_) => d.evaluate(x + 1, log),
                        Dist::Constant(_, _, _) => d.evaluate(0, log),
                        _ => d.evaluate(x, log),
                    }
                } else {
                    match d {
                        Dist::Categorical(prob_mass) => {
                            let p = prob_mass.get(0).unwrap();
                            (1. - p, *p)
                        }
                        Dist::Constant(_, _, p) => match is_inverse {
                            true => (*p, 1.),
                            false => (*p, 1. - p),
                        },
                        _ => (0., 0.),
                    }
                }
            }
        };

        (p0, p1)
    }
}

impl fmt::Display for DistLink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            DistLink::Counted(d) | DistLink::Indexed(d) => {
                write!(f, "{}", d)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_tuple_nearly_eq(a: (f64, f64), b: (f64, f64), epsilon: f64) {
        assert_relative_eq!(a.0, b.0, epsilon = 0.01);
        assert_relative_eq!(a.1, b.1, epsilon = 0.01);
    }

    #[test]
    fn test_distribution_constant() {
        assert_eq!(Dist::Constant(0, 1, 1.0).evaluated(1, false), (1.0, 1.0));
        assert_eq!(Dist::Constant(0, 1, 0.5).evaluated(1, false), (0.5, 0.5));
        assert_eq!(Dist::Constant(0, 1, 0.5).evaluated(2, false), (0.0, 0.0));
    }

    #[test]
    #[rustfmt::skip]
    fn test_distribution_constant_log() {
        assert_tuple_nearly_eq(Dist::Constant(0, 1, 1.0).evaluated(1, true), (0., 0.), 0.01);
        assert_tuple_nearly_eq(Dist::Constant(0, 1, 0.5).evaluated(1, true), (-0.69, -0.69), 0.01);
        assert_tuple_nearly_eq(Dist::Constant(0, 1, 0.5).evaluated(2, true), (0., 0.), 0.01);
    }

    #[test]
    fn test_distribution_exactly_times() {
        assert_eq!(Dist::ExactlyTimes(2).evaluated(0, false), (1.0, 0.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluated(1, false), (1.0, 0.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluated(2, false), (0.0, 1.0));
        assert_eq!(Dist::ExactlyTimes(2).evaluated(3, false), (0.0, 0.0));
    }

    #[test]
    #[rustfmt::skip]
    fn test_distribution_geometric_1_or_more() {
        assert_eq!(Dist::PGeometric(1, u64::MAX, 0.5).evaluated( 0, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(1, u64::MAX, 0.5).evaluated( 1, false), (0.5, 0.5));
        assert_eq!(Dist::PGeometric(1, u64::MAX, 0.5).evaluated( 2, false), (0.75, 0.25));
    }

    #[test]
    #[rustfmt::skip]
    fn test_distribution_geometric_2_or_more() {
        assert_eq!(Dist::PGeometric(2, u64::MAX, 0.5).evaluated(0, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(2, u64::MAX, 0.5).evaluated(1, false), (1.0, 0.0));
        assert_eq!(Dist::PGeometric(2, u64::MAX, 0.5).evaluated(2, false), (0.5, 0.5));
    }

    #[test]
    fn test_distribution_binomial_degenerate() {
        // p = 0, the distribution is concentrated at 0
        assert_eq!(Dist::PBinomial(0, 0, 0.0).evaluated(0, false), (0.0, 1.0));
        assert_eq!(Dist::PBinomial(0, 0, 0.0).evaluated(1, false), (0.0, 0.0));
        assert_eq!(Dist::PBinomial(0, 0, 0.0).evaluated(2, false), (0.0, 0.0));
        assert_eq!(Dist::PBinomial(0, 0, 0.0).evaluated(5, false), (0.0, 0.0));

        // p = 1, the distribution is concentrated at n
        assert_eq!(Dist::PBinomial(0, 1, 1.0).evaluated(1, false), (0.0, 1.0));
        assert_eq!(Dist::PBinomial(0, 1, 1.0).evaluated(2, false), (0.0, 0.0));
        assert_eq!(Dist::PBinomial(0, 5, 1.0).evaluated(5, false), (0.0, 1.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_1() {
        assert_eq!(Dist::PBinomial(0, 1, 0.5).evaluated(0, false), (0.5, 0.5));
        assert_eq!(Dist::PBinomial(0, 1, 0.5).evaluated(1, false), (0.5, 0.5));
        assert_eq!(Dist::PBinomial(0, 1, 0.5).evaluated(2, false), (0.0, 0.0));
    }

    #[test]
    fn test_distribution_binomial_up_to_2() {
        use Dist::PBinomial;
        assert_eq!(PBinomial(0, 2, 0.5).evaluated(0, false), (0.75, 0.25));
        assert_eq!(PBinomial(0, 2, 0.5).evaluated(1, false), (0.5, 0.5));
        assert_eq!(PBinomial(0, 2, 0.5).evaluated(2, false), (0.75, 0.25));
        assert_eq!(PBinomial(0, 2, 0.5).evaluated(3, false), (0.0, 0.0));
    }

    #[test]
    fn test_distribution_bernoulli() {
        assert_eq!(Dist::PBernoulli(0, 1, 0.5).evaluated(0, false), (0.5, 0.5));
        assert_eq!(Dist::PBernoulli(0, 1, 0.5).evaluated(1, false), (0.5, 0.5));
        assert_eq!(Dist::PBernoulli(0, 2, 0.5).evaluated(2, false), (1.0, 0.0));
    }

    #[test]
    fn test_distribution_categorical() {
        let dist = Dist::Categorical(vec![0.5, 0.3, 0.2]);

        assert_eq!(dist.evaluated(0, false), (0.5, 0.5));
        assert_eq!(dist.evaluated(1, false), (0.7, 0.3));
        assert_eq!(dist.evaluated(2, false), (0.8, 0.2));
        assert_eq!(dist.evaluated(3, false), (1.0, 0.0));
    }

    #[test]
    #[rustfmt::skip]
    fn test_distribution_zipf() {
        // n_max = 1, the normalizing constant is 1
        let dist = Dist::PZipf(0, 1, 1.0);
        assert_eq!(dist.evaluated(1, false), (1. - (1. / 1.), (1. / 1.)));
        assert_eq!(dist.evaluated(2, false), (1. - (1. / 2.), (1. / 2.)));

        // n_max = 2, the normalizing constant is 3/2 = 1.5
        let dist = Dist::PZipf(0, 2, 1.0);
        assert_eq!(dist.evaluated(1, false), (1. - (1. / 1.) / 1.5, (1. / 1.) / 1.5));
        assert_eq!(dist.evaluated(2, false), (1. - (1. / 2.) / 1.5, (1. / 2.) / 1.5));
    }
}
