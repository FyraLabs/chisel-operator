use std::env;

use chisel_operator::daemon;
use color_eyre::Result;
use tracing::info;
// use opentelemetry::sdk::export::metrics::StdoutExporterBuilder;
// use opentelemetry_api::trace::{
//     noop::{NoopTracer, NoopTracerProvider},
//     TracerProvider,
// };
// use tracing::info;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};
// Main entrypoint for operator

// TODO: OpenTelemetry is broken

// async fn init_tracer() -> opentelemetry::sdk::trace::Tracer {
//     let mut pipeline = opentelemetry_otlp::new_pipeline()
//         .tracing()
//         .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
//             opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
//                 "service.name",
//                 "chisel-operator",
//             )]),
//         ));

//     if let Ok(otlp_endpoint) = std::env::var("OPENTELEMETRY_ENDPOINT_URL") {
//         let channel = tonic::transport::Channel::from_shared(otlp_endpoint)
//             .unwrap()
//             .connect()
//             .await
//             .unwrap();

//         pipeline = pipeline.with_exporter(
//             opentelemetry_otlp::new_exporter()
//                 .tonic()
//                 .with_channel(channel),
//         )
//     } else {
//         pipeline = pipeline.with_exporter(opentelemetry_otlp::new_exporter().tonic())
//     }

//     pipeline
//         .install_batch(opentelemetry::runtime::Tokio)
//         .unwrap()
// }

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    let logger_env = env::var("LOGGER").unwrap_or_else(|_| "logfmt".to_string());

    let logfmt_logger = tracing_logfmt::layer().boxed();

    let pretty_logger = tracing_subscriber::fmt::layer().pretty().boxed();

    let json_logger = tracing_subscriber::fmt::layer().json().boxed();

    let compact_logger = tracing_subscriber::fmt::layer().compact().boxed();

    let logger = match logger_env.as_str() {
        "logfmt" => logfmt_logger,
        "pretty" => pretty_logger,
        "json" => json_logger,
        "compact" => compact_logger,
        _ => logfmt_logger,
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))?
        .add_directive("tower=off".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("kube_client=info".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("tokio_util=error".parse().unwrap());

    // let telemetry = tracing_opentelemetry::layer().with_tracer(init_tracer().await);
    let collector = Registry::default()
        // .with(telemetry)
        .with(logger)
        .with(env_filter);
    tracing::subscriber::set_global_default(collector)?;

    info!(
        "Fyra Labs Chisel Operator, version {}",
        env!("CARGO_PKG_VERSION")
    );
    info!("Starting up...");

    daemon::run().await
}
