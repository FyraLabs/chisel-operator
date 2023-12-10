use std::collections::BTreeMap;

use crate::cloud::digitalocean::DigitalOceanProvisioner;
use crate::cloud::CloudProvider;
use color_eyre::Result;
use k8s_openapi::api::core::v1::Secret;
use kube::{Api, CustomResource, core::ObjectMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::debug;
pub const EXIT_NODE_NAME_LABEL: &str = "chisel-operator.io/exit-node-name";
pub const EXIT_NODE_PROVISIONER_LABEL: &str = "chisel-operator.io/exit-node-provider";

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



impl ExitNode {
    /// for cloud provisioning: returns the name of the secret containing the cloud provider auth token
    ///
    /// if not exists, generates a new name using the ExitNode name
    pub fn get_secret_name(&self) -> String {
        match &self.spec.auth {
            Some(auth) => auth.clone(),
            None => format!("{}-auth", self.metadata.name.as_ref().unwrap()),
        }
    }
    
    pub fn get_external_host(&self) -> String {
        match &self.spec.external_host {
            Some(host) => host.clone(),
            None => {
                // check if status.ip exists
                // if it does, use that
                // otherwise use self.host
                match &self.status {
                    Some(status) => status.ip.clone(),
                    None => self.spec.host.clone(),
                }
            }
        }
    }

    /// returns the host
    pub async fn get_host(&self) -> String {
        // check if status.ip exists
        // if it does, use that
        // otherwise use self.host

        let client = kube::Client::try_default().await.unwrap();

        // ok, let's ask the API for reliable data

        let exit_nodes: Api<ExitNode> = Api::namespaced(client.clone(), &self.metadata.namespace.as_ref().unwrap().clone());

        let exit_node = exit_nodes.get(&self.metadata.name.as_ref().unwrap()).await.unwrap();

        debug!("ExitNode: {:#?}", exit_node.status);
        match &exit_node.status {
            Some(status) => status.ip.clone(),
            None => exit_node.spec.host.clone(),
        }
    }

    /// For cloud provisioning:
    /// 
    /// Generates a new secret with the `auth` key containing the auth string for chisel in the same namespace as the ExitNode
    pub async fn generate_secret(&self, password: String) -> Result<Secret> {
        let secret_name = self.get_secret_name();

        let auth_tmpl = format!("{}:{}", "chisel", password);

        let mut map = BTreeMap::new();
        map.insert(String::from("auth"), auth_tmpl);

        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(secret_name.clone()),
                namespace: self.metadata.namespace.clone(),
                ..Default::default()
            },
            string_data: Some(map),
            ..Default::default()
        };

        let client = kube::Client::try_default().await?;

        // add secret to k8s

        let secret_api = Api::<Secret>::namespaced(client.clone(), &self.metadata.namespace.as_ref().unwrap().clone());

        // force overwrite

        if let Ok(_existing_secret) = secret_api.get(&secret_name).await {
            debug!("Secret already exists, deleting");
            secret_api.delete(&secret_name, &Default::default()).await?;
        }

        let secret = secret_api.create(&kube::api::PostParams::default(), &secret).await?;

        Ok(secret)
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
