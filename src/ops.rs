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
pub struct ExitNodeSpec {
    pub host: String,
    pub port: u16,
    pub auth: Option<String>,
}

// #[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
// pub struct TunnelService {
//     pub local_host: String,
//     pub remote_port: u16,
//     pub remote_host: Option<String>,
//     pub custom_remote_host: bool,
// }

// impl TunnelService {
//     pub fn new(local_host: String, remote_port: u16) -> Self {
//         Self {
//             local_host,
//             remote_port,
//         }
//     }
//     pub fn to_connstring(&self) -> String {
//         let remote_host = if let Some(host) = self.remote_host.as_ref() {
//             host
//         } else {
//             "R"
//         };

//         let remote = format!("{}:{}", remote_host, self.remote_port);

//         format!("{}:{}", remote, self.local_host)
//     }
// }

// #[derive(Serialize, Deserialize, Debug, Default, Clone, JsonSchema, CustomResource)]
// #[kube(
//     group = "chisel-operator.io",
//     version = "v1",
//     kind = "RemoteTunnel",
//     status = "RemoteTunnelStatus",
//     plural = "remotetunnels",
//     singular = "tunnel",
//     shortname = "tun"
// )]
// #[serde(rename_all = "camelCase")]
// pub struct RemoteTunnelSpec {
//     pub hosts: Vec<TunnelService>,
//     // default host
// }

// #[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
// pub struct RemoteTunnelStatus {
//     pub ready: bool,
//     pub ip: Option<String>,
// }

// impl RemoteTunnelSpec {
//     /// Create new RemoteTunnel
//     pub fn new(local_host: String, remote_port: u16, remote_host: Option<String>) -> Self {
//         Self {
//             hosts: vec![TunnelService {
//                 local_host,
//                 remote_port,
//                 remote_host,
//                 custom_remote_host: false,
//             }],
//         }
//     }

//     /// Converts current RemoteTunnel into a chisel remote tunnel connection string
//     pub fn to_connstring(&self) -> String {
//         let remote_host = if let Some(host) = self.remote_host.as_ref() {
//             host
//         } else {
//             "R"
//         };

//         let remote = format!("{}:{}", remote_host, self.remote_port);

//         format!("{}:{}", remote, self.local_host)
//     }
// }

// pub fn generate_args(node: &ExitNodeSpec, tunnels: &[RemoteTunnelSpec]) -> Vec<String> {
//     let mut args = vec!["client".to_string()];

//     // Add node host and port
//     args.push(format!("{}:{}", node.host, node.port));

//     for tunnel in tunnels {
//         args.push(tunnel.to_connstring());
//     }
//     args
// }

// #[test]
// fn test_connstr() {
//     let remote_tunnel = RemoteTunnelSpec::new("localhost:8080".to_string(), 8080, None);
//     assert_eq!(remote_tunnel.to_connstring(), "R:8080:localhost:8080");

//     let remote_tunnel = RemoteTunnelSpec::new(
//         "localhost:8080".to_string(),
//         8080,
//         Some("example.com".to_string()),
//     );
//     assert_eq!(
//         remote_tunnel.to_connstring(),
//         "example.com:8080:localhost:8080"
//     );
// }
