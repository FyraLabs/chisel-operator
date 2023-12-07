use crate::cloud::digitalocean::DigitalOceanProvisioner;
use crate::cloud::CloudProvider;
use k8s_openapi::api::core::v1::Secret;
use kube::{CustomResource, Api};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use color_eyre::Result;

#[derive(Serialize, Deserialize, Debug, CustomResource, Clone, JsonSchema)]
#[kube(
    group = "chisel-operator.io",
    version = "v1",
    kind = "ExitNode",
    singular = "exitnode",
    struct = "ExitNode",
    status = "ExitNodeStatus",
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]

pub struct ExitNodeStatus {
    pub provider: String,
    pub name: String,
    // pub password: String,
    pub ip: String,
    pub id: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct LinodeProvisioner {
    pub auth: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AWSProvisioner {
    pub auth: String,
}

#[derive(Serialize, Deserialize, Debug, CustomResource, Clone, JsonSchema)]
#[kube(
    group = "chisel-operator.io",
    version = "v1",
    kind = "ExitNodeProvisioner",
    singular = "exitnodeprovisioner",
    struct = "ExitNodeProvisioner",
    namespaced
)]
/// ExitNodeProvisioner is a custom resource that represents a Chisel exit node provisioner on a cloud provider.
pub enum ExitNodeProvisionerSpec {
    DigitalOcean(DigitalOceanProvisioner),
    Linode(LinodeProvisioner),
    AWS(AWSProvisioner),
}


pub trait ProvisionerSecret {
    fn find_secret(&self) -> Result<Option<String>>;
}

impl ExitNodeProvisionerSpec {
    pub fn as_string(&self) -> String {
        match self {
            ExitNodeProvisionerSpec::DigitalOcean(_) => "digitalocean".to_string(),
            ExitNodeProvisionerSpec::Linode(_) => "linode".to_string(),
            ExitNodeProvisionerSpec::AWS(_) => "aws".to_string(),
        }
    }


}

impl ExitNodeProvisioner {
    pub async fn find_secret(&self) -> Result<Option<Secret>> {
        let secret_name = match &self.spec {
            ExitNodeProvisionerSpec::DigitalOcean(a) => a.auth.clone(),
            ExitNodeProvisionerSpec::Linode(a) => a.auth.clone(),
            ExitNodeProvisionerSpec::AWS(a) => a.auth.clone(),
        };

        // Find a k8s secret with the name of the secret reference

        let client = kube::Client::try_default().await?;

        let secret = Api::<Secret>::namespaced(
            client.clone(),
            &self.metadata.namespace.as_ref().unwrap().clone(),
        );

        let secret = secret.get(&secret_name).await?;

        Ok(Some(secret))
    }
}