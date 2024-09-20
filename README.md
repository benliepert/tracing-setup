# Tracing Setup
A crate to abstract top level configuration/boilerplate required when using the [tracing]() crate.

# Examples
This is a really simple crate. You just need to create a `TracingConfig` and pass it to `init_tracing()`
```rs
use tracing_setup::{TracingConfig, TracingMode, init_tracing};
let config = TracingConfig {
    tracing_mode: TracingMode::ConsoleAndFile,
    env_filter: Some("my_crate=debug".to_string()),
    json: false, // we don't want json formatting
    log_dir: "./logs".to_string(), // logs will be created here
    ansi_file: false, // we don't want ANSI colors in the file logs
    ansi_console: true, // we want ANSI colors in the console logs
}

// since we want to log to a file, `_guard` is a `tracing_appender::WorkerGuard` that will flush all pending logs
// when dropped.
let _guard = init_tracing(config);

// and we're off!
```
Additionally, you can enable the `jaeger` feature to use `TracingMode::JaegerLive`, which will stream data to a jaeger instance specified by `TracingConfig::jaeger_hostname`.