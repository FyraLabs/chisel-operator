use color_eyre::Result;
use kube::CustomResourceExt;
// use opentelemetry::sdk::export::metrics::StdoutExporterBuilder;
// use opentelemetry_api::trace::{
//     noop::{NoopTracer, NoopTracerProvider},
//     TracerProvider,
// };
// use tracing::info;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};
// Main entrypoint for operator
mod daemon;
mod deployment;
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

    let logger = tracing_subscriber::fmt::layer().json();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // let telemetry = tracing_opentelemetry::layer().with_tracer(init_tracer().await);
    let collector = Registry::default()
        // .with(telemetry)
        .with(logger)
        .with(env_filter);
    tracing::subscriber::set_global_default(collector).unwrap();

    // create crd

    println!("Creating CRD");
    println!(
        "crd for node:\n{}",
        serde_yaml::to_string(&ops::ExitNode::crd()).unwrap()
    );
    // println!(
    //     "crd for tunnel:\n{}",
    //     serde_yaml::to_string(&ops::RemoteTunnel::crd()).unwrap()
    // );
    daemon::run().await?;
    Ok(())
}
