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

pub(crate) use self::args::SchemaArgs;
use super::super::CliError;
use mockql_core::MinifyExt;
use mockql_core::SchemaTypes;
use mockql_core::load_schema;
use reqwest::Client;

pub(crate) async fn exec_schema(args: SchemaArgs) -> Result<(), CliError> {
  let SchemaArgs {
    minify,
    schema,
    graphql_url,
    headers,
    timeout,
  } = args;

  let client = Client::builder()
    .timeout(timeout)
    .build()
    .map_err(|e| CliError::ClientInit(e.to_string()))?;

  let schema = load_schema(schema.as_deref(), graphql_url.as_ref(), &headers, &client).await?;

  if minify {
    println!("{}", SchemaTypes::new(schema.into_inner().types).minify());
  } else {
    println!("{}", schema.serialize());
  }

  Ok(())
}
