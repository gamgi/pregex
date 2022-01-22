#![allow(unused_imports)]
#![feature(hash_drain_filter)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
use {
    clap::{App, Arg, ArgMatches},
    log::Level,
    std::error::Error,
    std::io::{self, prelude::*, BufReader, Cursor, Read},
    std::process::exit,
};

mod ast;
mod cli;
mod distribution;
mod nfa;
mod parser;
mod runner;
mod state;
mod utils;

pub type Result<T> = ::std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    env_logger::init();
    let matches = cli::options().get_matches();
    let config = cli::parse_options(matches)?;

    let asts = parser::parse(&config.pattern)?;
    let nfa = nfa::asts_to_nfa(asts);

    let reader = input_reader(&config)?;
    for line in reader.lines() {
        match line {
            Err(err) => {
                eprintln!("Failed to read input: {}", err);
                exit(1);
            }
            Ok(input_string) => match runner::matches(&nfa, &input_string) {
                true => println!("{}", input_string),
                false => {}
            },
        }
    }

    Ok(())
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
        None => Box::new(Cursor::new(config.input_string.to_string())),
    };

    Ok(BufReader::new(reader))
}
