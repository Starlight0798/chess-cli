//! 工具模块
//!
//! 包含通用的 Result 类型定义、重导出常用库以及日志宏定义。

pub type Result<T> = anyhow::Result<T>;
pub use anyhow::{Context, anyhow};
pub use crossterm::style::Stylize;
pub use hashbrown::HashMap;
pub use std::{
    convert::TryFrom,
    env::{current_exe, var},
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
};
pub use tokio::{io::AsyncBufReadExt, runtime::Runtime};

/// 初始化日志系统
///
/// 在调试模式下，日志将写入 `cli.log` 文件。
pub fn init_logger() -> Result<()> {
    #[cfg(debug_assertions)]
    {
        use std::{
            fs::{self, OpenOptions},
            sync::{Mutex, OnceLock},
        };
        use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*};

        static LOG_FILE: &str = "cli.log";
        fs::remove_file(LOG_FILE).ok();

        OnceLock::new().get_or_init(|| {
            let log_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(LOG_FILE)
                .expect("Failed to open log file");

            let filter = EnvFilter::builder()
                .with_default_directive(tracing::Level::DEBUG.into())
                .from_env_lossy();

            let file_layer = fmt::layer()
                .with_writer(Mutex::new(log_file))
                .with_ansi(false)
                .with_filter(filter);

            Registry::default().with(file_layer).init();
        });
    }
    Ok(())
}

/// 记录 INFO 级别日志
#[macro_export]
macro_rules! log_info {
    ($($arg:expr),* $(,)?) => {
        #[cfg(debug_assertions)]
        {
            use std::panic::Location;
            let location = Location::caller();
            $(
                tracing::info!(
                    "[{}:{}] {} = {:#?}",
                    location.file(),
                    location.line(),
                    stringify!($arg),
                    $arg
                );
            )*
        }
    };
}

/// 记录 WARN 级别日志
#[macro_export]
macro_rules! log_warn {
    ($($arg:expr),* $(,)?) => {
        #[cfg(debug_assertions)]
        {
            use std::panic::Location;
            let location = Location::caller();
            $(
                tracing::warn!(
                    "[{}:{}] {} = {:#?}",
                    location.file(),
                    location.line(),
                    stringify!($arg),
                    $arg
                );
            )*
        }
    };
}

/// 记录 ERROR 级别日志
#[macro_export]
macro_rules! log_error {
    ($($arg:expr),* $(,)?) => {
        #[cfg(debug_assertions)]
        {
            use std::panic::Location;
            let location = Location::caller();
            $(
                tracing::error!(
                    "[{}:{}] {} = {:#?}",
                    location.file(),
                    location.line(),
                    stringify!($arg),
                    $arg
                );
            )*
        }
    };
}

/// 记录 DEBUG 级别日志
#[macro_export]
macro_rules! log_dbg {
    ($($arg:expr),* $(,)?) => {
        #[cfg(debug_assertions)]
        {
            use std::panic::Location;
            let location = Location::caller();
            $(
                tracing::debug!(
                    "[{}:{}] {} = {:#?}",
                    location.file(),
                    location.line(),
                    stringify!($arg),
                    $arg
                );
            )*
        }
    };
}

pub use crate::log_info;
