#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![deny(unsafe_code)]
#![forbid(deprecated)]

mod utils;
mod game;
mod cli;
mod engine;

use crate::{
    cli::interface::run_interactive_loop,
    utils::*,
};

fn main() -> Result<()> {
    let rt: tokio::runtime::Runtime = tokio::runtime::Runtime::new()?;
    rt.block_on(run_interactive_loop())
}
