#[cfg(feature = "jaeger")]
pub mod jaeger_impl {
    use crate::tracing_setup::get_env_filter;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;
    use tracing_subscriber::{prelude::*, EnvFilter};

    const DEFAULT_JAEGER_PORT: u16 = 6831;

    #[cfg(feature = "jaeger")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct JaegerCfg {
        /// The hostname of the Jaeger instance
        pub jaeger_hostname: String,
    }

    #[cfg(feature = "jaeger")]
    impl Default for JaegerCfg {
        fn default() -> Self {
            JaegerCfg {
                jaeger_hostname: "localhost".to_string(),
            }
        }
    }

    // You can specify a more detailed envfilter like this:
    // "info,eframe=warn,shalias_ui=trace,shalias=trace"
    pub fn init_jaeger(config: &JaegerCfg, env_filter: &Option<String>) {
        let filter_layer = EnvFilter::new(get_env_filter(env_filter));
        let fmt_layer = tracing_subscriber::fmt::Layer::default();

        let endpt = format!("{}:{DEFAULT_JAEGER_PORT}", config.jaeger_hostname);

        // setup the default registry
        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer);

        let jaeger_tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name("sb-usb")
            .with_endpoint(endpt)
            .install_simple()
            .unwrap();
        let jaeger_layer = tracing_opentelemetry::layer().with_tracer(jaeger_tracer);

        // Finalize the registry with a custom layer
        registry.with(jaeger_layer).init();
    }

    /// Wait for Jaeger to start. Otherwise nothing will pick up the trace info
    /// Blocks until a jaeger instance is found.
    pub fn wait_for_jaeger(hostname: &str) {
        // This is the Web UI endpoint
        let endpoint = format!("{hostname}:16686");
        while TcpStream::connect(&endpoint).is_err() {
            println!("Waiting for Jaeger to start...");
            thread::sleep(Duration::from_secs(1));
        }
        println!("Found running Jaeger instance!");
    }
}
