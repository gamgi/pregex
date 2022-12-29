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
