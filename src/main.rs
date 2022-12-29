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
            // TODO move debug visualize here, and use match_likelihood_stats or soething to get state out?
            Ok(input) => match regex::match_likelihood(&nfa, &input, config.visualize) {
                // Some(p) => println!("{:.5}\t{}", p, input),
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
