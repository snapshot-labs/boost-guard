use crate::{ServerError, HUB_URL};
use ::axum::extract::Json;
use axum::response::IntoResponse;
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateVoucherResponse {
    // TODO: should we include ID of request?
    pub signature: String,
    pub boost_id: String,
    pub user: String,
    pub proposal_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateVoucherParams {
    pub proposal_id: String,
    pub voter_address: String,
    pub boosts: Vec<(String, String)>,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/proposal_query.graphql",
    response_derives = "Debug"
)]
struct ProposalQuery;

// Receives proposal_id, voter_address, and boost_id
// Queries graph to get boost info?
// Have to check proposal's type: needs to be single-choice or basic, else error -> DONE
// Have to check
// Boost info needed: eligiblity_criteria (incentive or bribe?)
pub async fn create_voucher_handler(
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let requests: Vec<CreateVoucherParams> = serde_json::from_value(p).expect("params");

    let client = reqwest::Client::new();
    for request in requests {
        check_proposal_type(client.clone(), &request.proposal_id).await?;
    }

    // Query the hub to get info about the user's vote
    let response = CreateVoucherResponse::default();
    Ok(Json(response))
}

async fn check_proposal_type(
    client: reqwest::Client,
    proposal_id: &str,
) -> Result<(), ServerError> {
    let variables = proposal_query::Variables {
        id: proposal_id.to_owned(),
    };

    let request_body = ProposalQuery::build_query(variables);

    let res = client.post(HUB_URL).json(&request_body).send().await?;
    let response_body: GraphQLResponse<proposal_query::ResponseData> = res.json().await?;
    let proposal: proposal_query::ProposalQueryProposal = response_body
        .data
        .ok_or("missing data from the hub")?
        .proposal
        .ok_or("missing proposal data from the hub")?;

    let proposal_type = proposal.type_.ok_or("missing proposal type from the hub")?;

    if (proposal_type != "single-choice") && (proposal_type != "basic") {
        return Err(ServerError::ErrorString(format!(
            "`{proposal_type:}` proposals are not eligible for boosting"
        )));
    }

    Ok(())
}
