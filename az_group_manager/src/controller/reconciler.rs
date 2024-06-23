use super::azure_group_manager_crd::AzureGroupManager;
use super::azure_group_members::{get_members, GroupResponse};
use super::error::{Error, Result};
use az_group_crd::{AzureGroup, AzureGroupSpec, AzureGroupStatus};
use chrono::prelude::*;
use futures::StreamExt;
use kube::api::{Patch, PatchParams};
use kube::runtime::reflector::Lookup;
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

    match get_members(&ctx.cli_args, &manager.spec.group_uid).await {
        Ok(group_response) => create_azure_group_resource(&group_response, &manager, &ctx.k8s_client).await?,
        Err(err) => return Err(err),
    };

    Ok(Action::requeue(Duration::from_secs(ctx.cli_args.reconcile_time)))
}

fn error_policy(_object: Arc<AzureGroupManager>, err: &Error, ctx: Arc<ReconcileContext>) -> Action {
    error!("ERROR POLICY: {}", err.to_string());
    Action::requeue(Duration::from_secs(ctx.cli_args.retry_time))
}

/// Creates an AzureGroup resource in Kubernetes using the provided `GroupResponse` and `AzureGroupManager`.
///
/// Arguments
///   * `g_response` - A reference to the `GroupResponse` containing the specifications for the Azure Group to be created.
///   * `manager` - A reference to the `AzureGroupManager` for getting owner reference and namepace.
///   * `k8s_client` - A reference to the `kube::Client` used to interact with the Kubernetes API server.
///
/// Returns a `Result<AzureGroup>` containing the Kubernetes resource.
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
    let result = patch_azure_group(&group_slug_name, &az_group_api, az_group).await?;

    patch_azure_group_status(&group_slug_name, &az_group_api).await?;

    return Ok(result);
}

async fn patch_azure_group_status(obj_name: &str, az_group_api: &Api<AzureGroup>) -> Result<AzureGroup> {
    let status = AzureGroupStatus {
        last_update: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let patch = json!({"status": status});

    let status = az_group_api
        .patch_status(obj_name, &PatchParams::default(), &Patch::Merge(&patch))
        .await?;

    return Ok(status);
}

async fn patch_azure_group(
    obj_name: &str,
    az_group_api: &Api<AzureGroup>,
    az_group: AzureGroup,
) -> Result<AzureGroup> {
    let result = az_group_api
        .patch(
            obj_name,
            &PatchParams::apply("azure-group-controller").force(),
            &Patch::Apply(&az_group),
        )
        .await
        .map_err(Error::AzureGroupCreationFailed)?;

    return Ok(result);
}
