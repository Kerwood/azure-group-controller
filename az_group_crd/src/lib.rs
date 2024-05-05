#[macro_use]
extern crate serde_derive;
use kube::CustomResource;
use kube::CustomResourceExt;
use schemars::JsonSchema;

#[derive(CustomResource, Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[kube(
    group = "kerwood.github.com",
    version = "v1",
    kind = "AzureGroup",
    namespaced,
    printcolumn = r#"{"name":"COUNT", "type":"string", "jsonPath":".spec.count"}"#,
    printcolumn = r#"{"name":"ID", "type":"string", "jsonPath":".spec.id"}"#,
    // printcolumn = r#"{"name":"LAST UPDATE", "type":"string", "jsonPath":".status.lastUpdate"}"#,
    // printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[kube(status = "AzureGroupStatus")]
#[serde(rename_all = "camelCase")]
pub struct AzureGroupSpec {
    pub id: String,
    pub members: Vec<Member>,
    pub count: usize,
    pub display_name: String,
    pub description: Option<String>,
    pub mail: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AzureGroupStatus {
    pub last_update: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub display_name: String,
    pub mail: String,
}

pub fn print_crd() -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&AzureGroup::crd())
}
