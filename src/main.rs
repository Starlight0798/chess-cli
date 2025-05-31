#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![deny(unsafe_code)]
#![forbid(deprecated)]

mod utils;
mod game;
mod engine;
mod cli;

use crate::{
    cli::interface::run,
    utils::*,
};

fn main() -> Result<()> {
    init_logger()?;
    let rt: Runtime = Runtime::new()?;
    #[cfg(debug_assertions)]
    {
        rt.block_on(cli::interface::run())?;
    }
    #[cfg(not(debug_assertions))]
    if let Err(e) = rt.block_on(cli::interface::run()) {
        let _ = cli::display::cleanup_terminal();
        eprintln!("程序错误: {}", e);
    }
    Ok(())
}
