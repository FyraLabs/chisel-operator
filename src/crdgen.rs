use kube::CustomResourceExt;

mod cloud;
mod ops;

fn main() {
    print!("{}", serde_yaml::to_string(&ops::ExitNode::crd()).unwrap());
    print!(
        "{}",
        serde_yaml::to_string(&ops::ExitNodeProvisioner::crd()).unwrap()
    )
}
