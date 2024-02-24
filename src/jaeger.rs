#[cfg(feature = "jaeger")]
pub mod jaeger_impl {
    use crate::{env_filter, TracingConfig};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;
    use tracing_subscriber::{prelude::*, EnvFilter};

    const DEFAULT_JAEGER_PORT: u16 = 6831;

    // You can specify a more detailed envfilter like this:
    // "info,eframe=warn,shalias_ui=trace,shalias=trace"
    pub fn init_jaeger(config: TracingConfig) {
        let filter_layer = EnvFilter::new(env_filter(&config));
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
        // This is the Web UI endpoint. I don't know how to test that the UDP port at 6831 is up
        let endpoint = format!("{hostname}:16686");
        while TcpStream::connect(&endpoint).is_err() {
            println!("Waiting for Jaeger to start...");
            thread::sleep(Duration::from_secs(1));
        }
        println!("Found running Jaeger instance!");
    }
}
