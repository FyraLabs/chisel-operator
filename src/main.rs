use color_eyre::Result;
// use opentelemetry::sdk::export::metrics::StdoutExporterBuilder;
// use opentelemetry_api::trace::{
//     noop::{NoopTracer, NoopTracerProvider},
//     TracerProvider,
// };
// use tracing::info;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};
// Main entrypoint for operator
mod cloud;
mod daemon;
mod deployment;
mod error;
mod ops;

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

    let logger = tracing_logfmt::layer();
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

    // let telemetry = tracing_opentelemetry::layer().with_tracer(init_tracer().await);
    let collector = Registry::default()
        // .with(telemetry)
        .with(logger)
        .with(env_filter);
    tracing::subscriber::set_global_default(collector)?;

    daemon::run().await
}
