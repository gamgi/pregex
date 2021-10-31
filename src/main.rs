#![allow(unused_imports)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

use log::Level;

mod ast;
mod nfa;
mod parser;
mod runner;

fn main() {
    env_logger::init();
    println!("Hello, world!");
}
