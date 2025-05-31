pub type Result<T> = anyhow::Result<T>;
pub use anyhow::{Context, anyhow};
pub use hashbrown::{HashMap, HashSet};
pub use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Lines, stdin},
    process::{Child, Command, ChildStdout, ChildStdin},
    time::{sleep, Duration},
    runtime::Runtime,
    spawn, select
};
pub use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{DisableMouseCapture, EnableMouseCapture, read, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize, StyledContent},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
pub use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
    process::{Stdio, exit},
    fs::{read_to_string, create_dir_all},
    env::{var, current_exe},
    str::{FromStr, SplitWhitespace},
    convert::TryFrom,
};
pub use async_trait::async_trait;

pub fn init_logger() -> Result<()> {
    #[cfg(debug_assertions)]
    {
        use std::{fs::{self, OpenOptions}, sync::{OnceLock, Mutex}};
        use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

        static LOG_FILE: &str = "cli.log";
        fs::remove_file(LOG_FILE).ok();

        OnceLock::new().get_or_init(|| {
            let log_file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(LOG_FILE)
                .expect("Failed to open log file");

            let filter = EnvFilter::builder()
                .with_default_directive(tracing::Level::DEBUG.into())
                .from_env_lossy();

            let file_layer = fmt::layer()
                .with_writer(Mutex::new(log_file))
                .with_ansi(false)
                .with_filter(filter);

            Registry::default()
                .with(file_layer)
                .init();
        });
    }
    Ok(())
}

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

pub use crate::{log_dbg, log_info, log_warn, log_error};