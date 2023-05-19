use color_eyre::Result;
use kube::CustomResourceExt;
use tracing_subscriber::{EnvFilter, Registry, prelude::*};
// Main entrypoint for operator
mod daemon;
mod deployment;
mod ops;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    let logger = tracing_subscriber::fmt::layer().json();
    let env_filter = EnvFilter::try_from_default_env()
    .or_else(|_| EnvFilter::try_new("info"))
    .unwrap();

    let collector = Registry::default().with(logger).with(env_filter);
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
