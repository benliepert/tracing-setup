#[cfg(feature = "jaeger")]
use crate::jaeger::jaeger_impl::JaegerCfg;
use chrono::Local;
use std::fs::File;
use tracing::subscriber::set_global_default;
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter, FmtSubscriber,
};

pub const DEFAULT_SPAN_EVENTS: FmtSpan = FmtSpan::CLOSE;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Controls how tracing is configured at a high level
pub enum TracingMode {
    /// Log to the console only
    Console(ConsoleCfg),
    /// Log to a file only
    File(FileCfg),
    /// Log to both the console and a file
    ConsoleAndFile(ConsoleCfg, FileCfg),
    /// Log to a live Jaeger instance. This automatically logs to the console as well
    #[cfg(feature = "jaeger")]
    JaegerLive(JaegerCfg),
}

impl Default for TracingMode {
    fn default() -> Self {
        TracingMode::Console(ConsoleCfg::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleCfg {
    /// ANSI colors in the console
    ///
    /// See [`SubscriberBuilder::with_ansi`](tracing_subscriber::fmt::SubscriberBuilder::with_ansi)
    pub ansi_console: bool,
}

impl Default for ConsoleCfg {
    fn default() -> Self {
        ConsoleCfg { ansi_console: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCfg {
    /// When writing to a file, this is the directory to write to.
    pub log_dir: &'static str,

    /// ANSI colors in the file
    ///
    /// See [`SubscriberBuilder::with_ansi`](tracing_subscriber::fmt::SubscriberBuilder::with_ansi)
    pub ansi_file: bool,

    /// Lossy file writing
    ///
    /// See [`NonBlockingBuilder::lossy`](tracing_appender::non_blocking::NonBlockingBuilder::lossy)
    pub lossy_file: bool,
}

impl Default for FileCfg {
    fn default() -> Self {
        FileCfg {
            log_dir: "./logs",
            ansi_file: false,
            lossy_file: true,
        }
    }
}

/// Holds information about how tracing should be configured
#[derive(Debug, Default)]
pub struct TracingConfig {
    pub tracing_mode: TracingMode,

    /// Environment filter for tracing. This controls the log levels and modules that are logged.
    ///
    /// If None, uses the default env var with [from_default_env](tracing_subscriber::EnvFilter::from_default_env).
    /// See [EnvFilter]
    pub env_filter: Option<String>,

    /// Use JSON formatting. This applies to all tracing modes except Jaeger.
    pub json: bool,
}

/// Initialize the tracing system based on a config
///
/// Returns an option to a WorkerGuard, which will flush all pending logs when dropped.
/// This is specifically for writing to a file. For ['crate::TracingMode::Console`] and [`TracingMode::JaegerLive`], this always returns None.
pub fn init_tracing(config: TracingConfig) -> Option<WorkerGuard> {
    // this separation is necessary because adding layers changes the type of the subscriber,
    // so it's impossible to genericize and make this cleaner^
    match config.tracing_mode {
        TracingMode::Console(console_cfg) => {
            configure_console_logging(&console_cfg, config.env_filter, config.json);
            None
        }
        TracingMode::File(file_cfg) => {
            configure_file_logging(&file_cfg, config.env_filter, config.json)
        }
        TracingMode::ConsoleAndFile(console_cfg, file_cfg) => {
            configure_combined_logging(&console_cfg, &file_cfg, config.env_filter, config.json)
        }
        #[cfg(feature = "jaeger")]
        TracingMode::JaegerLive(jaeger_cfg) => {
            use crate::jaeger::jaeger_impl::*;
            wait_for_jaeger(&jaeger_cfg.jaeger_hostname); // block until jaeger is running
            println!("Initializing tracing for live streaming to Jaeger. Make sure you start the Jaeger Docker container.");
            init_jaeger(&jaeger_cfg, &config.env_filter);
            None
        }
    }
}

/// Only file logging will be configured
fn configure_file_logging(
    config: &FileCfg,
    env_filter: Option<String>,
    json: bool,
) -> Option<WorkerGuard> {
    let (non_blocking, guard) = file_writer(config);

    let subscriber_builder = FmtSubscriber::builder()
        .with_env_filter(get_env_filter(&env_filter))
        .with_span_events(DEFAULT_SPAN_EVENTS)
        .with_ansi(config.ansi_file)
        .with_writer(non_blocking);

    // since adding json formatting changes the type, some code needs to be duplicated
    if json {
        let subscriber = subscriber_builder.json().finish();
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let subscriber = subscriber_builder.finish();
        set_global_default(subscriber).expect("Failed to set global default");
    }

    Some(guard)
}

/// Only console logging will be configured
fn configure_console_logging(config: &ConsoleCfg, env_filter: Option<String>, json: bool) {
    // let format = fmt::format().json();
    let subscriber_builder = FmtSubscriber::builder()
        .with_env_filter(get_env_filter(&env_filter))
        .with_span_events(DEFAULT_SPAN_EVENTS)
        .with_ansi(config.ansi_console);

    // since adding json formatting changes the type, some code needs to be duplicated
    if json {
        let subscriber = subscriber_builder.json().finish();
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let subscriber = subscriber_builder.finish();
        set_global_default(subscriber).expect("Failed to set global default");
    }
}

/// Both console and file logging will be configured
// reward for whoever can make this cleaner
fn configure_combined_logging(
    console_cfg: &ConsoleCfg,
    file_cfg: &FileCfg,
    env_filter: Option<String>,
    json: bool,
) -> Option<WorkerGuard> {
    use tracing_subscriber::prelude::*;

    // File writer setup
    let (non_blocking, guard) = file_writer(file_cfg);
    let env_filter = EnvFilter::new(get_env_filter(&env_filter));

    // due to some complexities in the type system, this code is duplicated
    // the only difference is that when layers are added to the subscriber, if config.json
    // json formatting is used.
    if json {
        let stdout_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(console_cfg.ansi_console);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(file_cfg.ansi_file);
        let subscriber = tracing_subscriber::registry()
            .with(stdout_layer.json())
            .with(file_layer.json())
            .with(env_filter);
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let stdout_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(console_cfg.ansi_console);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(file_cfg.ansi_file);
        let subscriber = tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .with(env_filter);
        set_global_default(subscriber).expect("Failed to set global default");
    }
    Some(guard)
}

/// Return an environment filter based on the provided config.
/// If config.env_filter is None, the default filter will be used
/// See [`EnvFilter::from_default_env`](tracing_subscriber::EnvFilter::from_default_env)
pub(crate) fn get_env_filter(filter: &Option<String>) -> String {
    if let Some(filter) = &filter {
        filter.clone()
    } else {
        EnvFilter::from_default_env().to_string()
    }
}

/// Create a file writer based on a tracing config
///
/// The file writer will log tracing information to a file
fn file_writer(config: &FileCfg) -> (NonBlocking, WorkerGuard) {
    let filename = create_log_filename(config.log_dir);
    create_parent_directory(&filename).expect("Failed to create parent log directory");
    let file_writer = File::create(&filename).expect("Failed to create log file");
    NonBlockingBuilder::default()
        .lossy(config.lossy_file)
        .finish(file_writer)
}

/// Generate a log filename based on the current time
fn create_log_filename(log_dir: &str) -> String {
    let now = Local::now();
    format!("{}/log_{}.txt", log_dir, now.format("%m-%d-%Y_%H-%M-%S"))
}

/// Given a path, create the parent directory if it doesn't exist. Otherwise, do nothing.
fn create_parent_directory(path: &str) -> std::io::Result<()> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        // create the parent directory if it doesn't exist
        std::fs::create_dir_all(parent)
    } else {
        // no parent directory to create
        Ok(())
    }
}
