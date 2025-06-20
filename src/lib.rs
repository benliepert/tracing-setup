pub mod tracing_setup;

pub(crate) mod jaeger;

pub use tracing_setup::{init_tracing, ConsoleCfg, FileCfg, TracingConfig, TracingMode};

#[cfg(feature = "jaeger")]
pub use jaeger::jaeger_impl::{init_jaeger, wait_for_jaeger, JaegerCfg};

// re-exports
pub use tracing;
pub use tracing_appender;
