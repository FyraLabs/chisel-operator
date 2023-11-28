use async_trait::async_trait;
use digitalocean_rs::DigitalOceanApi;
use digitalocean_rs::DigitalOceanError;
use names::Generator;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;

/// Simple wrapper for names crate
pub fn generate_name() -> String {
    let mut generator = Generator::default();
    generator.next().unwrap()
}

mod cloud_init;
pub mod digitalocean;
mod pwgen;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum CloudProvider {
    DigitalOcean,
    Linode,
    AWS,
}
pub struct CloudExitNode {
    pub provider: CloudProvider,
    pub name: String,
    pub password: String,
    pub ip: String,
}

#[async_trait]
pub trait Provisioner {
    async fn create_exit_node(&self) -> color_eyre::Result<CloudExitNode>;
    async fn update_exit_node(&self, exit_node: CloudExitNode)
        -> color_eyre::Result<CloudExitNode>;
    async fn delete_exit_node(&self, exit_node: CloudExitNode) -> color_eyre::Result<()>;
}

// Each LB service binds to an exit node, which will be a many-to-one relationship
// An LB can annotate a specific exit node to bind to, or it can specify a provider to automatically provision a new exit node
// if no specific exit node is specified and a provider is not specified, then the first exit node returned by the K8S API will be used
// but if provider is specified, then a new exit node will be provisioned on that provider
// A provisioner can have many exit nodes that it manages
// each exit node can be manually managed or automatically managed by a provisioner
// you can request a new exit node from a provisioner by simply creating a LB service without specifying a specific exit node
// or you can create a new managed exit node

// Take LB1 which has annotation chisel-operator.io/cloud-provisioner: do
// Take LB2 which has annotation chisel-operator.io/cloud-provisioner: do ON A DIFFERENT PORT
// what if I want to use the same exit node for both LB1 and LB2?
// maybe we can introduce a new annotation chisel-operator.io/cloud-exit-node: <name>
// if two LBs have the same cloud-exit-node annotation, then they will use the same exit node, WHEN THE PROVISIONER IS THE SAME
