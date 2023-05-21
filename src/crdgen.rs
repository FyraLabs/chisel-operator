use kube::CustomResourceExt;

mod ops;

fn main() {
    print!("{}", serde_yaml::to_string(&ops::ExitNode::crd()).unwrap())
}
