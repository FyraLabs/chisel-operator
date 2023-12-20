use kube::CustomResourceExt;
use std::fs::File;
use std::io::prelude::*;

mod cloud;
mod daemon;
mod deployment;
mod error;
mod ops;

// todo: Make this a cargo xtask, maybe?

fn main() {
    let mut exit_node_provisioner = File::create("deploy/crd/exit-node-provisioner.yaml").unwrap();
    exit_node_provisioner
        .write_all(
            serde_yaml::to_string(&ops::ExitNodeProvisioner::crd())
                .unwrap()
                .as_bytes(),
        )
        .unwrap();

    let mut exit_node = File::create("deploy/crd/exit-node.yaml").unwrap();
    exit_node
        .write_all(
            serde_yaml::to_string(&ops::ExitNode::crd())
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}
