use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, CustomResource, Clone, JsonSchema)]
#[kube(
    group = "chisel-operator.io",
    version = "v1",
    kind = "ExitNode",
    singular = "exitnode",
    struct = "ExitNode",
    namespaced
)]
/// ExitNode is a custom resource that represents a Chisel exit node.
/// It will be used as the reverse proxy for all services in the cluster.
pub struct ExitNodeSpec {
    /// Hostname or IP address of the chisel server
    pub host: String,
    /// Control plane port of the chisel server
    pub port: u16,
    /// Optional authentication secret name to connect to the control plane
    pub auth: Option<String>,
}
