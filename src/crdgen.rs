use kube::CustomResourceExt;

mod cloud;
mod daemon;
mod deployment;
mod error;
mod ops;

// todo: Make this a cargo xtask, maybe?

fn main() {
    println!("{}", serde_yaml::to_string(&ops::ExitNode::crd()).unwrap());
    println!("---");
    println!(
        "{}",
        serde_yaml::to_string(&ops::ExitNodeProvisioner::crd()).unwrap()
    )
}
