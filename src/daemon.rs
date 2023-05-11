// Daemon module
// watch for changes in all LoadBalancer services and update the IP addresses

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{FieldsV1, ManagedFieldsEntry};
use kube::api::{ResourceExt, WatchEvent};
use kube::Resource;
use std::thread;
use std::time::Duration;

use k8s_openapi::api::core::v1::Service;
use kube::{
    api::{Api, ListParams, WatchParams},
    Client,
};

use crate::ops::{ExitNode, RemoteTunnel};

pub async fn process_svcs(mut service: Service) {
    let client = Client::try_default().await.unwrap();
    let tunnels: Api<ExitNode> = Api::all(client);

    let lp = ListParams::default().timeout(30);
    let node_list = tunnels.list(&lp).await.unwrap();

    // We will only be supporting 1 exit node for all services for now, so we can just grab the first one
    let node: &ExitNode = node_list.items.first().unwrap();

    // set managed fields if not already set
    let mut managed_fields = service.managed_fields_mut();

    // find managed field for this controller
    let managed_field = managed_fields.iter_mut().find(|mf| {
        mf.api_version == Some(RemoteTunnel::api_version(&()).to_string())
            && mf.manager == Some("chisel-operator".to_string())
    });
    if let None = managed_field {
        let mf = ManagedFieldsEntry {
            api_version: Some(RemoteTunnel::api_version(&()).to_string()),
            fields_type: Some("FieldsV1".to_string()),
            fields_v1: Some(FieldsV1(
                serde_json::json!({
                    "spec": {
                        "externalIPs": {}
                    },
                    "status": {
                        "loadBalancer": {
                            "ingress": {}
                        }
                    }
                })
                .clone(),
            )),
            subresource: Some("status".to_string()),
            manager: Some("chisel-operator".to_string()),
            operation: Some("Update".to_string()),
            ..Default::default()
        };
        managed_fields.push(mf);
    }
}

pub async fn run() -> color_eyre::Result<()> {
    let client = Client::try_default().await.unwrap();
    // watch for K8s service resources (default)
    let services: Api<Service> = Api::all(client);
    let lp = WatchParams::default()
        // .fields("spec.type=LoadBalancer")
        .timeout(0);
    loop {
        let mut stream = services.watch(&lp, "0").await?.boxed();
        while let Ok(Some(status)) = stream.try_next().await {
            match status {
                WatchEvent::Added(s) => println!("Added {:?}", s),
                WatchEvent::Modified(s) => println!("Modified: {:?}", s),
                WatchEvent::Deleted(s) => println!("Deleted {:?}", s),
                WatchEvent::Bookmark(_) => {}
                WatchEvent::Error(s) => println!("{:?}", s),
            }
        }
        thread::sleep(Duration::from_secs(30));
    }

    // Ok(())
}
