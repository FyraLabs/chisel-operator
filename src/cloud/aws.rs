use super::{cloud_init::generate_cloud_init_config, pwgen::generate_password, Provisioner};
use crate::{
    cloud::CHISEL_PORT,
    ops::{parse_provisioner_label_value, ExitNode, ExitNodeStatus, EXIT_NODE_PROVISIONER_LABEL},
};
use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::types::{Tag, TagSpecification};
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use base64::Engine;
use color_eyre::eyre::anyhow;
use k8s_openapi::api::core::v1::Secret;
use kube::ResourceExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

const DEFAULT_SIZE: &str = "t2.micro";
const UBUNTU_AMI_SSM_KEY: &str =
    "/aws/service/canonical/ubuntu/server/24.04/stable/current/amd64/hvm/ebs-gp2/ami-id";

fn default_size() -> String {
    String::from(DEFAULT_SIZE)
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct AWSProvisioner {
    /// Reference to a secret containing the AWS access key ID and secret access key, under the `access_key_id` and `secret_access_key` secret keys
    pub auth: String,
    /// Region ID for the AWS region to provision the exit node in
    /// See https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-regions-availability-zones.html
    pub region: String,
    /// Security group name to use for the exit node, uses the default security group if not specified
    pub security_group: Option<String>,
    /// Size for the EC2 instance
    /// See https://aws.amazon.com/ec2/instance-types/
    #[serde(default = "default_size")]
    pub size: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AWSIdentity {
    access_key_id: String,
    secret_access_key: String,
    pub region: String,
}

impl AWSIdentity {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
        }
    }
    /// Generate an AWS config from the access key ID and secret access key
    pub async fn generate_aws_config(self) -> color_eyre::Result<aws_config::SdkConfig> {
        // We use our own hyper client and TLS config in order to use webpki-roots instead of the system ones
        // This let's us use a sane set of root certificates instead of the ones that come with the OS
        let tls_connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_only()
            .enable_http1()
            .enable_http2()
            .build();

        let hyper_client = HyperClientBuilder::new().build(tls_connector);

        // set access key id and secret access key as environment variables
        std::env::set_var("AWS_ACCESS_KEY_ID", &self.access_key_id);
        std::env::set_var("AWS_SECRET_ACCESS_KEY", &self.secret_access_key);
        let region: String = self.region.clone();
        Ok(aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .http_client(hyper_client)
            .load()
            .await)
    }

    // This code is very unholy, but thanks to Jeff Bezos for making the AWS SDK so complicated
    pub fn from_secret(secret: &Secret, region: String) -> color_eyre::Result<Self> {
        let aws_access_key_id: String = secret
            .data
            .as_ref()
            .and_then(
                |f: &std::collections::BTreeMap<String, k8s_openapi::ByteString>| {
                    f.get("AWS_ACCESS_KEY_ID")
                },
            )
            .ok_or_else(|| anyhow!("AWS_ACCESS_KEY_ID not found in secret"))
            // into String
            .and_then(|key| String::from_utf8(key.clone().0.to_vec()).map_err(|e| e.into()))?;

        let aws_secret_access_key: String = secret
            .data
            .as_ref()
            .and_then(|f| f.get("AWS_SECRET_ACCESS_KEY"))
            .ok_or_else(|| anyhow!("AWS_SECRET_ACCESS_KEY not found in secret"))
            .and_then(|key| String::from_utf8(key.clone().0.to_vec()).map_err(|e| e.into()))?;

        Ok(Self {
            access_key_id: aws_access_key_id,
            secret_access_key: aws_secret_access_key,
            region,
        })
    }
}

#[async_trait]
impl Provisioner for AWSProvisioner {
    async fn create_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
        node_password: String,
    ) -> color_eyre::Result<ExitNodeStatus> {
        let provisioner = exit_node
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get(EXIT_NODE_PROVISIONER_LABEL))
            .ok_or_else(|| {
                anyhow!(
                    "No provisioner found in annotations for exit node {}",
                    exit_node.metadata.name.as_ref().unwrap()
                )
            })?;

        let cloud_init_config = generate_cloud_init_config(&node_password, CHISEL_PORT);
        let user_data = base64::engine::general_purpose::STANDARD.encode(cloud_init_config);

        let aws_api: aws_config::SdkConfig = AWSIdentity::from_secret(&auth, self.region.clone())?
            .generate_aws_config()
            .await?;

        let ssm_client = aws_sdk_ssm::Client::new(&aws_api);
        let parameter_response = ssm_client
            .get_parameter()
            .name(UBUNTU_AMI_SSM_KEY)
            .send()
            .await?;
        let ami = parameter_response.parameter.unwrap().value.unwrap();

        let ec2_client = aws_sdk_ec2::Client::new(&aws_api);

        let current_namespace = exit_node.namespace().unwrap();
        let (_provisioner_namespace, provsioner_name) =
            parse_provisioner_label_value(&current_namespace, provisioner);

        let name = format!(
            "{}-{}",
            provsioner_name,
            exit_node.metadata.name.as_ref().unwrap()
        );

        let tag_specification = TagSpecification::builder()
            .resource_type("instance".into())
            .tags(Tag::builder().key("Name").value(name.clone()).build())
            .build();

        let mut instance_builder = ec2_client
            .run_instances()
            .tag_specifications(tag_specification)
            .image_id(ami)
            .instance_type(self.size.as_str().into())
            .min_count(1)
            .max_count(1)
            .user_data(&user_data);

        if let Some(security_group) = &self.security_group {
            instance_builder = instance_builder.security_group_ids(security_group);
        }

        let instance_response = instance_builder.send().await?;

        let instance = instance_response
            .instances
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // TODO: Refactor this to run on a reconcile update instead
        let public_ip = loop {
            let describe_response = ec2_client
                .describe_instances()
                .instance_ids(instance.instance_id.clone().unwrap())
                .send()
                .await?;
            let reservation = describe_response
                .reservations
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            let instance = reservation.instances.unwrap().into_iter().next().unwrap();

            debug!(?instance, "Getting instance data");

            if let Some(ip) = instance.public_ip_address {
                break ip;
            } else {
                warn!("Waiting for instance to get IP address");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        };

        // let exit_node = ExitNodeStatus {
        //     name: name.clone(),
        //     ip: public_ip,
        //     id: Some(instance.instance_id.unwrap()),
        //     provider: provisioner.clone(),
        //     service_binding: vec![],
        // };
        let exit_node = ExitNodeStatus::new(
            provisioner.clone(),
            name.clone(),
            public_ip,
            // needless conversion?
            // todo: Clean this up, minor performance hit
            instance.instance_id,
        );

        Ok(exit_node)
    }

    async fn update_exit_node(
        &self,
        auth: Secret,
        exit_node: ExitNode,
        node_password: String,
    ) -> color_eyre::Result<ExitNodeStatus> {
        let aws_api: aws_config::SdkConfig = AWSIdentity::from_secret(&auth, self.region.clone())?
            .generate_aws_config()
            .await?;
        let ec2_client = aws_sdk_ec2::Client::new(&aws_api);

        let node = exit_node.clone();

        if let Some(ref status) = &exit_node.status {
            let instance_id = status.id.as_ref().ok_or_else(|| {
                anyhow!(
                    "No instance ID found in status for exit node {}",
                    node.metadata.name.as_ref().unwrap()
                )
            })?;

            let describe_response = ec2_client
                .describe_instances()
                .instance_ids(instance_id)
                .send()
                .await?;
            let reservation = describe_response
                .reservations
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            let instance = reservation.instances.unwrap().into_iter().next().unwrap();

            let mut status = status.clone();

            if let Some(ip) = instance.public_ip_address {
                status.ip = ip;
            }

            Ok(status)
        } else {
            warn!("No status found for exit node, creating new instance");
            // TODO: this should be handled by the controller logic
            return self.create_exit_node(auth, exit_node, node_password).await;
        }
    }

    async fn delete_exit_node(&self, auth: Secret, exit_node: ExitNode) -> color_eyre::Result<()> {
        let aws_api: aws_config::SdkConfig = AWSIdentity::from_secret(&auth, self.region.clone())?
            .generate_aws_config()
            .await?;
        let ec2_client = aws_sdk_ec2::Client::new(&aws_api);

        let instance_id = exit_node
            .status
            .as_ref()
            .and_then(|status| status.id.as_ref());

        if let Some(instance_id) = instance_id {
            ec2_client
                .terminate_instances()
                .instance_ids(instance_id)
                .send()
                .await?;
        }

        Ok(())
    }
}
