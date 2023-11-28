use kube::CustomResourceExt;

mod cloud;
mod ops;

fn main() {
    println!("{}", serde_yaml::to_string(&ops::ExitNode::crd()).unwrap());
    println!("---");
    println!(
        "{}",
        serde_yaml::to_string(&ops::ExitNodeProvisioner::crd()).unwrap()
    )
}
