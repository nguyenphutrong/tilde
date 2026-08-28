use anyhow::{Result, anyhow};
use async_trait::async_trait;
use cynic::QueryBuilder;
#[cfg(test)]
use mockall::automock;
use warp_graphql::queries::get_runners::{
    GetRunners, GetRunnersResult, GetRunnersVariables, Runner, RunnerSortBy,
};

use super::ServerApi;
use crate::server::graphql::{get_request_context, get_user_facing_error_message};

/// Client for runner discovery.
#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait FactoryClient: 'static + Send + Sync {
    async fn get_runners(&self, sort_by: Option<RunnerSortBy>) -> Result<Vec<Runner>>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl FactoryClient for ServerApi {
    async fn get_runners(&self, sort_by: Option<RunnerSortBy>) -> Result<Vec<Runner>> {
        let operation = GetRunners::build(GetRunnersVariables {
            request_context: get_request_context(),
            sort_by,
        });
        let response = self.send_graphql_request(operation, None).await?;
        match response.get_runners {
            GetRunnersResult::GetRunnersOutput(output) => Ok(output.runners),
            GetRunnersResult::UserFacingError(error) => {
                Err(anyhow!(get_user_facing_error_message(error)))
            }
            GetRunnersResult::Unknown => Err(anyhow!("failed to list runners")),
        }
    }
}
