use super::azure_group_manager_crd::AzureGroupManager;
use super::azure_group_members::{get_members, GroupResponse};
use super::error::{Error, Result};
use az_group_crd::{AzureGroup, AzureGroupSpec, AzureGroupStatus};
use chrono::prelude::*;
use futures::StreamExt;
use kube::api::{Patch, PatchParams};
use kube::{
    api::ObjectMeta,
    runtime::{
        controller::{Action, Controller},
        watcher,
    },
    Api, Client, Resource, ResourceExt,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct Args {
    pub azure_tenant_id: String,
    pub azure_client_id: String,
    pub azure_client_secret: String,
    pub reconcile_time: u64,
    pub retry_time: u64,
}

#[derive(Clone)]
struct ReconcileContext {
    cli_args: Args,
    k8s_client: Client,
}

pub async fn run(cli_args: Args) -> Result<(), kube::Error> {
    let k8s_client = Client::try_default().await?;
    let azure_groups_manager_api = Api::<AzureGroupManager>::all(k8s_client.clone());
    let azure_groups_api = Api::<AzureGroup>::all(k8s_client.clone());
    let ctx = ReconcileContext { cli_args, k8s_client };

    Controller::new(azure_groups_manager_api.clone(), Default::default())
        .owns(azure_groups_api, watcher::Config::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(ctx))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

async fn reconcile(manager: Arc<AzureGroupManager>, ctx: Arc<ReconcileContext>) -> Result<Action> {
    debug!("running reconcile for manager object: {}", manager.name_any());

    // let members = get_members(&ctx.cli_args, &manager.spec.group_uid).await;
    match get_members(&ctx.cli_args, &manager.spec.group_uid).await {
        Ok(group_response) => {
            create_azure_group_resource(&group_response, &manager, &ctx.k8s_client).await?;
            update_manager_status(&manager, &ctx.k8s_client).await?;
        }
        Err(err) => return Err(err),
    }

    Ok(Action::requeue(Duration::from_secs(ctx.cli_args.reconcile_time)))
}

fn error_policy(_object: Arc<AzureGroupManager>, err: &Error, ctx: Arc<ReconcileContext>) -> Action {
    error!("ERROR POLICY: {}", err.to_string());
    Action::requeue(Duration::from_secs(ctx.cli_args.retry_time))
}

async fn create_azure_group_resource(
    g_response: &GroupResponse,
    manager: &AzureGroupManager,
    k8s_client: &kube::Client,
) -> Result<AzureGroup> {
    let azure_spec: AzureGroupSpec = g_response.clone().try_into()?;
    let owner_ref = manager.controller_owner_ref(&()).unwrap();
    let namespace = manager
        .metadata
        .namespace
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let group_slug_name = g_response
        .slug_display_name()
        .ok_or_else(|| Error::MissingDisplayName(azure_spec.id.to_string()))?;

    let az_group = AzureGroup {
        metadata: ObjectMeta {
            name: Some(group_slug_name.clone()),
            owner_references: Some(vec![owner_ref.clone()]),
            ..ObjectMeta::default()
        },
        spec: azure_spec,
        status: None,
    };
    let az_group_api = Api::<AzureGroup>::namespaced(k8s_client.clone(), namespace);
    let result = az_group_api
        .patch(
            &group_slug_name,
            &PatchParams::apply("azure-group-controller").force(),
            &Patch::Apply(&az_group),
        )
        .await
        .map_err(Error::AzureGroupCreationFailed)?;
    return Ok(result);
}

async fn update_manager_status(
    manager: &AzureGroupManager,
    k8s_client: &kube::Client,
) -> Result<AzureGroupManager> {
    let namespace = manager
        .metadata
        .namespace
        .as_ref()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let az_group_manager_api = Api::<AzureGroupManager>::namespaced(k8s_client.clone(), namespace);

    let status = AzureGroupStatus {
        last_update: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let patch = json!({"status": status});

    let status = az_group_manager_api
        .patch_status(
            &manager.name_any(),
            &PatchParams::default(),
            &Patch::Merge(&patch),
        )
        .await?;

    return Ok(status);
}
