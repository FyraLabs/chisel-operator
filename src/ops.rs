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
    /// Optional real external hostname/IP of exit node
    /// If not provided, the host field will be used
    #[serde(default)]
    pub external_host: Option<String>,
    /// Control plane port of the chisel server
    pub port: u16,
    /// Optional but highly recommended fingerprint to perform host-key validation against the server's public key
    pub fingerprint: Option<String>,
    /// Optional authentication secret name to connect to the control plane
    pub auth: Option<String>,
    /// Optional boolean value for whether to make the exit node the default route for the cluster
    /// If true, the exit node will be the default route for the cluster
    /// default value is false
    #[serde(default)]
    pub default_route: bool,
}


impl ExitNodeSpec {
    /// Returns the external host if it exists, otherwise returns the host
    // jokes on you, This is actually used in the reconcile loop.
    // rustc is weird.
    #[allow(dead_code)]
    pub fn get_external_host(&self) -> String {
        match &self.external_host {
            Some(host) => host.clone(),
            None => self.host.clone(),
        }
    }
}