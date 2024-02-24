use chrono::Local;
use std::fs::File;
use tracing::subscriber::set_global_default;
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter, FmtSubscriber,
};
pub const DEFAULT_SPAN_EVENTS: FmtSpan = FmtSpan::CLOSE;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
/// Controls how tracing is configured at a high level
pub enum TracingMode {
    /// Log to the console only
    #[default]
    Console,
    /// Log to a file only
    File,
    /// Log to both the console and a file
    ConsoleAndFile,
    /// Log to a live Jaeger instance. This automatically logs to the console as well
    #[cfg(feature = "jaeger")]
    JaegerLive,
}

/// Holds information about how tracing should be configured
#[derive(Debug)]
pub struct TracingConfig {
    pub tracing_mode: TracingMode,

    /// Environment filter for tracing.
    ///
    /// See ['EnvFilter](tracing_subscriber::EnvFilter)
    pub env_filter: Option<String>,

    /// Use JSON formatting for logs. This applies to all tracing modes except Jaeger.
    pub json: bool,

    /// When writing to a file, this is the directory to write to.
    pub log_dir: String,

    /// ANSI colors in the file
    ///
    /// See [`SubscriberBuilder::with_ansi`](tracing_subscriber::fmt::SubscriberBuilder::with_ansi)
    pub ansi_file: bool,
    /// Lossy file writing
    ///
    /// See [`NonBlockingBuilder::lossy`](tracing_appender::non_blocking::NonBlockingBuilder::lossy)
    pub lossy_file: bool,

    /// ANSI colors in the console
    ///
    /// See [`SubscriberBuilder::with_ansi`](tracing_subscriber::fmt::SubscriberBuilder::with_ansi)
    pub ansi_console: bool,

    /// The hostname of the jaeger instance, if applicable
    #[cfg(feature = "jaeger")]
    pub jaeger_hostname: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        TracingConfig {
            tracing_mode: TracingMode::Console,
            env_filter: None,
            json: false,
            log_dir: "./logs".to_string(),
            ansi_file: false,
            lossy_file: true,
            ansi_console: true,
            #[cfg(feature = "jaeger")]
            jaeger_hostname: "localhost".to_string(),
        }
    }
}

/// Initialize the tracing system based on a config
///
/// Returns an option to a WorkerGuard, which will flush all pending logs when dropped.
/// This is specifically for writing to a file. When config.log_to_file is false, this will
/// return None.
pub fn init_tracing(config: TracingConfig) -> Option<WorkerGuard> {
    // this separation is necessary because adding layers changes the type of the subscriber,
    // so it's impossible to genericize and make this cleaner^
    match config.tracing_mode {
        TracingMode::Console => {
            configure_console_logging(config);
            None
        }
        TracingMode::File => configure_file_logging(config),
        TracingMode::ConsoleAndFile => configure_combined_logging(config),
        #[cfg(feature = "jaeger")]
        TracingMode::JaegerLive => {
            #[cfg(feature = "jaeger")]
            {
                use crate::jaeger::jaeger_impl::*;
                wait_for_jaeger(&config.jaeger_hostname); // block until jaeger is running
                println!("Initializing tracing for live streaming to Jaeger. Make sure you start the Jaeger Docker container.");
                init_jaeger(config);
            }
            #[cfg(not(feature = "jaeger"))]
            {
                println!("Jaeger tracing is not enabled. Please enable the 'jaeger' feature. The app will still run, but you won't see any output");
            }

            None
        }
    }
}

/// Return an environment filter based on the provided config.
/// If config.env_filter is None, the default filter will be used
/// See [`EnvFilter::from_default_env`](tracing_subscriber::EnvFilter::from_default_env)
pub fn env_filter(config: &TracingConfig) -> String {
    if let Some(filter) = &config.env_filter {
        filter.clone()
    } else {
        EnvFilter::from_default_env().to_string()
    }
}

/// Only file logging will be configured
fn configure_file_logging(config: TracingConfig) -> Option<WorkerGuard> {
    let (non_blocking, guard) = file_writer(&config);

    let subscriber_builder = FmtSubscriber::builder()
        .with_env_filter(env_filter(&config))
        .with_span_events(DEFAULT_SPAN_EVENTS)
        .with_ansi(config.ansi_file)
        .with_writer(non_blocking);

    // since adding json formatting changes the type, some code needs to be duplicated
    if config.json {
        let subscriber = subscriber_builder.json().finish();
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let subscriber = subscriber_builder.finish();
        set_global_default(subscriber).expect("Failed to set global default");
    }

    Some(guard)
}

/// Only console logging will be configured
fn configure_console_logging(config: TracingConfig) {
    // let format = fmt::format().json();
    let subscriber_builder = FmtSubscriber::builder()
        .with_env_filter(env_filter(&config))
        .with_span_events(DEFAULT_SPAN_EVENTS)
        .with_ansi(config.ansi_console);

    // since adding json formatting changes the type, some code needs to be duplicated
    if config.json {
        let subscriber = subscriber_builder.json().finish();
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let subscriber = subscriber_builder.finish();
        set_global_default(subscriber).expect("Failed to set global default");
    }
}

/// Both console and file logging will be configured
// reward for whoever can make this cleaner
fn configure_combined_logging(config: TracingConfig) -> Option<WorkerGuard> {
    use tracing_subscriber::prelude::*;
    // File writer setup
    let (non_blocking, guard) = file_writer(&config);

    // due to some complexities in the type system, this code is duplicated
    // the only difference is that when layers are added to the subscriber, if config.json
    // json formatting is used.
    if config.json {
        let stdout_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(config.ansi_console);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(config.ansi_file);
        let subscriber = tracing_subscriber::registry()
            .with(stdout_layer.json())
            .with(file_layer.json())
            .with(EnvFilter::new(env_filter(&config)));
        set_global_default(subscriber).expect("Failed to set global default");
    } else {
        let stdout_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(config.ansi_console);
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_span_events(DEFAULT_SPAN_EVENTS)
            .with_ansi(config.ansi_file);
        let subscriber = tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .with(EnvFilter::new(env_filter(&config)));
        set_global_default(subscriber).expect("Failed to set global default");
    }
    Some(guard)
}

/// Create a file writer based on a tracing config
///
/// The file writer will log tracing information to a file
fn file_writer(config: &TracingConfig) -> (NonBlocking, WorkerGuard) {
    let filename = create_log_filename(&config.log_dir);
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

/// Given a path, create the parent directory if it doesn't exist.
/// Otherwise, do nothing.
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
