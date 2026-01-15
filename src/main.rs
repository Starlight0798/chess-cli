//! Chess CLI 入口点
//!
//! 这个文件是应用程序的入口点，负责初始化运行时环境、日志系统，并启动 TUI 界面。

#![deny(unsafe_code)]
#![forbid(deprecated)]

mod engine;
mod game;
mod tui;
mod utils;

use crate::utils::*;

fn main() -> Result<()> {
    init_logger()?;
    let rt: Runtime = Runtime::new()?;

    // 启动 TUI 应用程序
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
