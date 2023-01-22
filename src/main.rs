#![allow(unused_imports)]
#![feature(hash_drain_filter)]

use {
    clap::Parser,
    itertools::Itertools,
    log::Level,
    statrs::distribution::{Bernoulli, Binomial, Discrete, Geometric},
    std::error::Error,
    std::io::{self, prelude::*, BufReader, Cursor, Read},
    std::process::exit,
};

mod ast;
mod charclass;
mod cli;
mod distribution;
mod nfa;
mod parser;
mod regex;
mod regex_state;
mod visualization;

use crate::cli::Config;

pub type Result<T> = ::std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let config = Config::parse();
    env_logger::init();
    let nfa = compile(&config.pattern)?;
    let reader = input_reader(&config)?;

    for line in reader.lines() {
        match line {
            Ok(input) => match regex::match_likelihood(&nfa, &input, config.visualize) {
                Some(p) => println!("{:.5}\t{}", p, input),
                None => {}
            },
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

pub fn compile(source: &str) -> Result<Vec<nfa::State>> {
    Ok(nfa::asts_to_nfa(parser::parse(source)?))
}

/// Get input reader based on config
///
/// If input_file is set, it has precedence over input_string
/// If input_file is "-", returns reade from stdin.
/// Otherwise, returns reader from input_string.
fn input_reader(config: &cli::Config) -> Result<BufReader<Box<dyn Read>>> {
    use std::fs::File;

    let reader: Box<dyn Read> = match &config.input_file {
        Some(input_file) => match input_file.as_str() {
            "-" => Box::new(io::stdin()),
            _ => Box::new(File::open(input_file)?),
        },
        None => Box::new(Cursor::new(
            config
                .input_string
                .as_ref()
                .map(|s| s.to_string())
                .expect("input string to have been specified"),
        )),
    };

    Ok(BufReader::new(reader))
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use regex::match_likelihood;

    #[test]
    fn test_basic() {
        let nfa = compile("abc").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(
            match_likelihood(&nfa, &"abcd".to_string(), false),
            Some(1.0)
        );
    }

    #[test]
    fn test_basic_anchor() {
        let nfa = compile("^abc$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"abcd".to_string(), false), None);
    }

    #[test]
    fn test_literal_escape() {
        let nfa = compile(r"^a\\db$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"a0b".to_string(), false), None);
        assert_eq!(
            match_likelihood(&nfa, &"a\\db".to_string(), false),
            Some(1.0)
        );
        assert_eq!(match_likelihood(&nfa, &"a\\ddb".to_string(), false), None);
    }

    #[test]
    fn test_quantifier_zero() {
        let nfa = compile("^ab{0}$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"aa".to_string(), false), None);
    }

    #[test]
    fn test_quantifier_zero_constant() {
        let nfa = compile("^ab{0~Const(0.5)}$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"aa".to_string(), false), None);

        let nfa = compile("^a\\d{0~Const(0.5)}$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"a0".to_string(), false), None);
    }

    #[test]
    #[rustfmt::skip]
    fn test_quantifier_geo() {
        let nfa = compile("^a{5~Geo(0.5)}$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"aaaa".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"aaaaa".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"aaaaaa".to_string(), false), Some(0.25));
    }

    #[test]
    #[rustfmt::skip]
    fn test_quantifier_zipf() {
        let nfa = compile("^a{2~Zipf(1.0)}$").unwrap();
        let harmonic_number_2 = 3. / 2.;
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some((1. / 1.) / harmonic_number_2));
        assert_eq!(match_likelihood(&nfa, &"aa".to_string(), false), Some((1. / 2.) / harmonic_number_2));
        assert_eq!(match_likelihood(&nfa, &"aaa".to_string(), false), Some((1. / 3.) / harmonic_number_2));
    }

    #[test]
    #[rustfmt::skip]
    fn test_class_zipf() {
        // n = 2, 1st is 2/3 and the 2nd is 1/3
        // s = 1.0, normalizer is the (non-generalized) harmonic number
        let nfa = compile("^[ab~Zipf(1.0)]$").unwrap();
        let harmonic_number_2 = 3. / 2.;
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some((1. / 1.) / harmonic_number_2));
        assert_eq!(match_likelihood(&nfa, &"b".to_string(), false), Some((1. / 2.) / harmonic_number_2));
        assert_eq!(match_likelihood(&nfa, &"c".to_string(), false), None);
    }

    #[test]
    fn test_class_geo() {
        let nfa = compile("^[abc~Geo(0.5)]$").unwrap();
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"b".to_string(), false), Some(0.25));
        assert_eq!(match_likelihood(&nfa, &"c".to_string(), false), Some(0.125));
    }

    #[test]
    fn test_class_ber() {
        let nfa = compile("^[abc~Ber(0.5)]$").unwrap();
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"b".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"c".to_string(), false), None);
    }

    #[test]
    fn test_class_bin() {
        let nfa = compile("^[abc~Bin(0.5)]$").unwrap();
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.25));
        assert_eq!(match_likelihood(&nfa, &"b".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"c".to_string(), false), Some(0.25));
    }

    #[test]
    fn test_class_nested_geo() {
        let nfa = compile(r"^[a\d~Geo(0.5)]$").unwrap();
        assert_eq!(match_likelihood(&nfa, &"a".to_string(), false), Some(0.5));
        assert_eq!(match_likelihood(&nfa, &"0".to_string(), false), Some(0.25));
        assert_eq!(match_likelihood(&nfa, &"1".to_string(), false), Some(0.125));
        assert_eq!(match_likelihood(&nfa, &"b".to_string(), false), None);
    }

    #[test]
    fn test_dot() {
        let nfa = compile("^a.c$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"abcd".to_string(), false), None);
    }

    #[test]
    fn test_dot_quantifier_plus() {
        let nfa = compile("^a.+c$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(
            match_likelihood(&nfa, &"abbc".to_string(), false),
            Some(1.0)
        );
        assert_eq!(match_likelihood(&nfa, &"abcd".to_string(), false), None);
    }

    #[test]
    fn test_dot_quantifier_question() {
        let nfa = compile("^a.?c$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"ac".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"abbc".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abcd".to_string(), false), None);
    }

    #[test]
    fn test_dot_quantifier_exact() {
        let nfa = compile("^a.{2}c$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ac".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"ab".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), None);
        assert_eq!(
            match_likelihood(&nfa, &"abbc".to_string(), false),
            Some(1.0)
        );
        assert_eq!(match_likelihood(&nfa, &"abbbc".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abcd".to_string(), false), None);
    }

    #[test]
    fn test_short_class() {
        let nfa = compile("^a\\dc$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ac".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"a1c".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"a2c".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"a12c".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), None);
    }

    #[test]
    fn test_long_class() {
        let nfa = compile("^a[bc]c$").unwrap();

        assert_eq!(match_likelihood(&nfa, &"ac".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abc".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"acc".to_string(), false), Some(1.0));
        assert_eq!(match_likelihood(&nfa, &"adc".to_string(), false), None);
        assert_eq!(match_likelihood(&nfa, &"abcc".to_string(), false), None);
    }
}
