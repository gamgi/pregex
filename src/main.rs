#![allow(unused_imports)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

mod ast;
mod nfa;
mod parser;
mod runner;

fn main() {
    println!("Hello, world!");
}
