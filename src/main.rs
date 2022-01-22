#![allow(unused_imports)]
#![feature(hash_drain_filter)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
use clap::{App, Arg, ArgMatches};

use log::Level;

mod ast;
mod config;
mod distribution;
mod nfa;
mod parser;
mod runner;
mod state;
mod utils;
use config::{parse_options, PATTERN_OPTION, STRING_OPTION};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), String> {
    env_logger::init();
    let options = App::new("Pregex")
        .version(VERSION)
        .arg(Arg::with_name("pattern").help("Regexp pattern").index(1))
        .arg(
            Arg::with_name("string")
                .help("String to match against")
                .index(2),
        )
        .get_matches();

    let config = parse_options(options)?;
    let asts = parser::parse(&config.pattern).unwrap_or_else(|error| {
        panic!("{}", error);
    });

    let nfa = nfa::asts_to_nfa(asts);
    match runner::matches(&nfa, &config.string) {
        true => println!("{}", config.string),
        false => {}
    };

    Ok(())
}
