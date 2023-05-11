use kube::CustomResourceExt;
use color_eyre::Result;
// Main entrypoint for operator
mod ops;
mod daemon;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    // create crd

    println!("Creating CRD");
    println!(
        "crd for node:\n{}",
        serde_yaml::to_string(&ops::ExitNode::crd()).unwrap()
    );
    println!(
        "crd for tunnel:\n{}",
        serde_yaml::to_string(&ops::RemoteTunnel::crd()).unwrap()
    );
    daemon::run().await?;
    Ok(())
}
