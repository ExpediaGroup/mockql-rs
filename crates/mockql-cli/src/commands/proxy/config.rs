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

use mockql_core::ProviderConfig;
use reqwest::Url;
use std::sync::Arc;

pub(crate) type SharedProxyConfig = Arc<ProxyConfig>;

pub(crate) struct ProxyConfig {
  pub(crate) graphql_url: Url,
  pub(crate) provider: ProviderConfig,
}

impl ProxyConfig {
  pub(crate) fn new_shared(graphql_url: Url, provider: ProviderConfig) -> SharedProxyConfig {
    Arc::new(Self { graphql_url, provider })
  }
}
