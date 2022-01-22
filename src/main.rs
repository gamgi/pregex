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
    match runner::matches(&nfa, &config.input_string) {
        true => println!("{}", config.input_string),
        false => {}
    };

    Ok(())
}
