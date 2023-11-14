#![warn(clippy::pedantic)]

mod engine;
mod grammar;

extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate from_pest;
extern crate pest_ast;

pub use engine::TestEngine;
