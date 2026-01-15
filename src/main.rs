#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![deny(unsafe_code)]
#![forbid(deprecated)]

mod utils;
mod game;
mod engine;
mod tui;

use crate::utils::*;

fn main() -> Result<()> {
    init_logger()?;
    let rt: Runtime = Runtime::new()?;
    
    // 使用新的 TUI 界面
    if let Err(e) = rt.block_on(async {
        let mut app = tui::app::App::new()?;
        app.run().await
    }) {
        println!("Error: {}", e);
        #[cfg(debug_assertions)]
        return Err(e);
    }
    Ok(())
}
