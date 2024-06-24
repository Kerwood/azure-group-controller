use super::error::Error as CrateError;
use super::error::Result;
use super::reconciler::Args;
use az_group_crd::{AzureGroupSpec, Member};
use azure_identity::client_credentials_flow;
use slug::slugify;
use tracing::{debug, error};
use url::Url;

// Struct used for authentication against the Azure Graph API.
#[derive(Debug, Clone)]
pub struct AzureCredentials {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

// The response object from the Graph API.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfoResponse {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub mail: Option<String>,
}

// The response object from the Graph API.
#[derive(Debug, Deserialize, Clone)]
pub struct GroupResponse {
    #[serde(rename = "value")]
    pub members: Vec<GroupResponseMember>,
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub mail: Option<String>,
}

impl GroupResponse {
    pub fn slug_display_name(&self) -> Option<String> {
        if self.display_name.is_none() {
            return None;
        }
        self.display_name.clone().map(|x| slugify(x))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponseMember {
    pub id: String,
    pub display_name: String,
    pub mail: Option<String>,
}

impl TryFrom<GroupResponseMember> for Member {
    type Error = CrateError;
    fn try_from(value: GroupResponseMember) -> Result<Self, Self::Error> {
        if value.mail.is_none() {
            return Err(CrateError::IntoMemberFailed(format!(
                "propery 'mail' is missing on {}",
                value.display_name
            )));
        }

        Ok(Self {
            id: value.id,
            display_name: value.display_name,
            mail: value.mail.unwrap(),
        })
    }
}

impl TryFrom<GroupResponse> for AzureGroupSpec {
    type Error = CrateError;
    fn try_from(group_response: GroupResponse) -> Result<Self, Self::Error> {
        if group_response.id.is_none() {
            return Err(CrateError::IntoAzureGroupSpecFailed(
                "field 'id' is None.".to_string(),
            ));
        }

        if group_response.display_name.is_none() {
            let message = format!(
                "field 'display_name' is None on group: {}",
                group_response.id.unwrap()
            );
            return Err(CrateError::IntoAzureGroupSpecFailed(message));
        }

        let (members_res, fails): (Vec<_>, Vec<_>) = group_response
            .members
            .into_iter()
            .map(|x| x.try_into())
            .partition(Result::is_ok);

        fails.into_iter().for_each(|x| error!("{}", x.unwrap_err()));
        let members: Vec<Member> = members_res.into_iter().map(Result::unwrap).collect();

        let result = Self {
            id: group_response.id.unwrap(),
            count: members.len(),
            members,
            description: group_response.description,
            mail: group_response.mail,
            display_name: group_response.display_name.unwrap(),
        };

        debug!("Converted GroupResponse to AzureGroupSpec {:?}", result);

        Ok(result)
    }
}

pub async fn get_members(args: &Args, group_uuid: &String) -> Result<GroupResponse> {
    let http_client = azure_core::new_http_client();

    // This will give you the final token to use in authorization.
    let token = client_credentials_flow::perform(
        http_client.clone(),
        &args.azure_client_id,
        &args.azure_client_secret,
        &["https://graph.microsoft.com/.default"],
        &args.azure_tenant_id,
    )
    .await?;

    // Get all member is the group
    let members_url = Url::parse(&format!(
        "https://graph.microsoft.com/v1.0/groups/{}/members",
        group_uuid
    ))?;
    let members_resp = reqwest::Client::new()
        .get(members_url)
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .send()
        .await?
        .error_for_status()?
        .json::<GroupResponse>()
        .await?;

    // Get basic information about the group.
    let group_info_url = Url::parse(&format!("https://graph.microsoft.com/v1.0/groups/{}", group_uuid))?;
    let group_info_resp = reqwest::Client::new()
        .get(group_info_url)
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .send()
        .await?
        .error_for_status()?
        .json::<GroupInfoResponse>()
        .await?;

    let result = GroupResponse {
        id: Some(group_uuid.to_string()),
        display_name: group_info_resp.display_name,
        mail: group_info_resp.mail,
        description: group_info_resp.description,
        ..members_resp
    };

    Ok(result)
}
