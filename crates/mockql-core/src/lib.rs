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

//! `mockql-core` provides the building blocks behind `mockql`.
//!
//! It analyzes GraphQL operations for `@mock` directives, builds provider prompts,
//! optionally executes upstream GraphQL requests, and merges partial mock responses
//! back into standard GraphQL Response.
#![deny(missing_docs)]

mod planner;

mod graphql;
mod llm_provider;
mod schema;
mod service;
mod upstream;

/// A typed HTTP header pair.
pub type Header = (reqwest::header::HeaderName, reqwest::header::HeaderValue);

pub use graphql::GraphQLRequest;
pub use graphql::GraphQLResponse;
pub use graphql::SchemaTypes;
pub use graphql::minify::MinifyExt;
pub use graphql::mock_response_prompt::DEBUG_KEY;
pub use graphql::mock_response_prompt::MOCK_RESPONSE_EXTENSION_KEY;
pub use graphql::mock_response_prompt::MockResponsePrompt;
pub use graphql::mock_response_prompt::REASONING_KEY;
pub use graphql::mock_response_prompt::SCHEMA_EXTENSION_KEY;
pub use graphql::mock_response_prompt::SerializationFormat;
pub use graphql::response_merger::ResponseMerger;
pub use graphql::response_merger::ResponseMergerError;
pub use graphql::response_validator::ResponseValidationError;
pub use graphql::schema_filter::SchemaFilter;
pub use graphql::schema_filter::SchemaFilterError;
pub use llm_provider::ProviderConfig;
pub use llm_provider::ProviderError;
pub use llm_provider::cli::ClaudeCliProvider;
pub use llm_provider::cli::CliProvider;
pub use llm_provider::cli::CodexCliProvider;
pub use llm_provider::cli::OpenCodeCliProvider;
pub use llm_provider::generate_mock_response;
pub use llm_provider::http::HttpProvider;
pub use llm_provider::http::gemini::GeminiCompatibleHttpProvider;
pub use llm_provider::http::github_copilot::GithubCopilotHttpProvider;
pub use planner::SplitError;
pub use schema::IntrospectionError;
pub use schema::SchemaLoadError;
pub use schema::load_schema;
pub use schema::to_schema_with_mock_directive;
pub use service::MockPlan;
pub use service::MockService;
pub use service::MockServiceRequest;
pub use service::ServiceError;
pub use upstream::UpstreamError;
