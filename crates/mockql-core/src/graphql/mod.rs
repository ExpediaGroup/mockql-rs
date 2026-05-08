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

//! GraphQL-specific helpers used by the mock engine.

/// Schema minification helpers for prompt generation.
pub mod minify;
/// `@mock` directive extraction and classification.
pub mod mock_directive_extractor;
/// Prompt structures and serialization helpers for mock providers.
pub mod mock_response_prompt;
/// Operation filtering helpers for partial mock execution.
pub mod operation_field_filter;
/// Response merging helpers for combining upstream and mocked data.
pub mod response_merger;
/// Schema filtering helpers for producing prompt-local schemas.
pub mod schema_filter;
/// Core GraphQL request and response types.
pub mod types;

pub use types::GraphQLRequest;
pub use types::GraphQLResponse;
pub use types::SchemaTypes;
