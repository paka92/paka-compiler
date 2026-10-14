//! The front end of the compiler: the syntax tree, its parser and the built-in table. The tools
//! that read programs without compiling them (the exit gate) use it as a library.
#![deny(clippy::float_arithmetic)]

pub mod ast;
pub mod builtins;
pub mod coverage;
pub mod diagnostic;
pub mod json;
mod lexer;
pub mod parser;
