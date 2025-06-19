# Tracing Setup
Abstracts configuration/boilerplate required when setting up the [tracing](https://docs.rs/tracing/latest/tracing/) crate.

# Why?
I found myself re-implementing a basic setup of tracing in every Rust project. Now, you only need to call 1 function with a simple high-level config. No more worrying about layers and subscribers!

# Who is this for?
If you want to get tracing up and running _fast_ and don't really care are customization beyond seeing stuff in a console/file, this is for you.

If you're interested in customizations, you'll probably want to stick to the public APIs. But I'm open to contributions!

# Examples
The fastest way to get up and running is to create a `TracingConfig` and pass it to `init_tracing()`
```rs
use tracing_setup::{TracingConfig, init_tracing, TracingMode, FileCfg, ConsoleCfg};
use tracing_setup::tracing::info; // tracing_setup re-exports tracing

// The default config logs to your console with ANSI colors.
// Note: the return value of init_tracing is only relevant when writing to a file
init_tracing(TracingConfig::default().with_env_filter("my_crate=debug"));
info!("This is an info message"); // will be logged to the console with colors
```
Or, you can customize a bit
```rs
use tracing_setup::{TracingConfig, init_tracing, TracingMode, FileCfg, ConsoleCfg};
use tracing_setup::tracing::info; // tracing_setup re-exports tracing

let config = TracingConfig {
    mode: TracingMode::ConsoleAndFile(
        ConsoleCfg {
            ansi_colors: false // use ANSI colors?
        },
        FileCfg {
            log_dir: "./logs", // logs will be saved here and automatically named log_{timestamp}.txt
            ansi_colors: false, // use ANSI colors?
            lossy_writing: false
        }
    ),
    // the env filter gives you per-crate and per-module control over tracing levels
    env_filter: Some("tracing_setup=trace".to_string()),
    json: true, // should json formatting be used?
};

// When `_guard` is dropped, file writes will be flushed. This ensures all data is written out
let _guard = init_tracing(config);

info!("This is an info message"); // will be logged to the console and a file under ./logs
```
