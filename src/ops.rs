use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct Auth {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, CustomResource, Clone, JsonSchema)]
#[kube(
    group = "chisel-operator.io",
    version = "v1",
    kind = "ExitNode",
    plural = "exitnodes",
    singular = "exitnode",
    shortname = "exit"
)]
pub struct ExitNodeSpec {
    pub host: String,
    pub port: u16,
    pub auth: Option<Auth>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, JsonSchema, CustomResource)]
#[kube(
    group = "chisel-operator.io",
    version = "v1",
    kind = "RemoteTunnel",
    status = "RemoteTunnelStatus",
    plural = "remotetunnels",
    singular = "tunnel",
    shortname = "tun"
)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTunnelSpec {
    pub local_host: String,
    pub remote_port: u16,
    pub remote_host: Option<String>,
    // default host
    pub custom_remote_host: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct RemoteTunnelStatus {
    pub ready: bool,
    pub ip: Option<String>,
}

impl RemoteTunnelSpec {
    /// Create new RemoteTunnel
    pub fn new(local_host: String, remote_port: u16, remote_host: Option<String>) -> Self {
        let custom_remote_host = remote_host.is_some();
        Self {
            local_host,
            remote_port,
            remote_host,
            custom_remote_host,
        }
    }

    /// Converts current RemoteTunnel into a chisel remote tunnel connection string
    pub fn to_connstring(&self) -> String {
        let remote_host = if let Some(host) = self.remote_host.as_ref() {
            host
        } else {
            "R"
        };

        let remote = format!("{}:{}", remote_host, self.remote_port);

        format!("{}:{}", remote, self.local_host)
    }
}

pub fn generate_args(node: &ExitNodeSpec, tunnels: &[RemoteTunnelSpec]) -> Vec<String> {
    let mut args = vec!["client".to_string()];

    // Add node host and port
    args.push(format!("{}:{}", node.host, node.port));

    for tunnel in tunnels {
        args.push(tunnel.to_connstring());
    }
    args
}

#[test]
fn test_connstr() {
    let remote_tunnel = RemoteTunnelSpec::new("localhost:8080".to_string(), 8080, None);
    assert_eq!(remote_tunnel.to_connstring(), "R:8080:localhost:8080");

    let remote_tunnel = RemoteTunnelSpec::new(
        "localhost:8080".to_string(),
        8080,
        Some("example.com".to_string()),
    );
    assert_eq!(
        remote_tunnel.to_connstring(),
        "example.com:8080:localhost:8080"
    );
}
