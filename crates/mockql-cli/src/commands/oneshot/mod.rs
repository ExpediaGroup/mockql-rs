// Copyright 2026 Expedia, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod args;

pub(crate) use self::args::OneshotArgs;
use super::super::CliError;
use crate::commands::shared::read_json_map;
use crate::commands::shared::read_schema_extension;
use mockql_core::MockService;
use mockql_core::MockServiceRequest;
use mockql_core::load_schema;
use reqwest::Client;
use std::fs;
use std::sync::Arc;

pub(crate) async fn exec_oneshot(args: OneshotArgs) -> Result<(), CliError> {
  let OneshotArgs {
    operation,
    graphql_url,
    schema,
    operation_name,
    variables,
    schema_extension,
    graphql_headers,
    timeout,
    format,
    transport,
  } = args;

  let client = Client::builder()
    .timeout(timeout)
    .build()
    .map_err(|e| CliError::ClientInit(e.to_string()))?;

  let schema = Arc::new(load_schema(schema.as_deref(), Some(&graphql_url), &graphql_headers, &client).await?);

  let response = MockService::new(schema, format.into(), client.clone())
    .execute(
      MockServiceRequest::builder()
        .provider(transport.into_provider_config(&timeout, client))
        .operation(fs::read_to_string(&operation)?)
        .and_operation_name(operation_name)
        .variables(read_json_map(variables.as_deref())?)
        .and_schema_extension(read_schema_extension(schema_extension.as_deref())?)
        .graphql_url(graphql_url)
        .graphql_headers(graphql_headers)
        .build(),
    )
    .await?;

  println!("{}", serde_json::to_string(&response)?);
  Ok(())
}
