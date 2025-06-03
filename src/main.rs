#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![deny(unsafe_code)]
#![forbid(deprecated)]

mod utils;
mod game;
mod engine;
mod cli;

use crate::utils::*;

fn main() -> Result<()> {
    init_logger()?;
    execute!(stdout(), Clear(ClearType::All))?;
    let rt: Runtime = Runtime::new()?;
    if let Err(e) = rt.block_on(cli::interface::run()) {
        let _ = cli::display::cleanup_terminal();
        cli::display::show_error(&format!("{}", e).to_string())?;
        
        #[cfg(debug_assertions)]
        return Err(e);
    }
    Ok(())
}
