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

//! Helpers for filtering operations into mocked and passthrough subsets.

use apollo_compiler::ExecutableDocument;
use apollo_compiler::request::RequestError;
use apollo_compiler::validation::Valid;
use std::collections::HashSet;

mod exclude;
mod include;

/// Filters an executable GraphQL document by response paths.
pub struct OperationFieldFilter<'a> {
  document: &'a Valid<ExecutableDocument>,
  paths: HashSet<String>,
}

/// Controls whether matching paths are kept or removed.
#[derive(Debug, Copy, Clone)]
pub enum FilterMode {
  /// Once a field is kept, its children need to be kept
  Include,
  /// Once a field is removed, its children are removed
  Exclude,
}

impl<'a> OperationFieldFilter<'a> {
  /// Creates a new field filter for the provided document and response paths.
  pub fn new(document: &'a Valid<ExecutableDocument>, paths: &[String]) -> Self {
    Self {
      document,
      paths: paths.iter().cloned().collect(),
    }
  }

  /// Produces a filtered copy of the executable document.
  pub fn filter(
    &self,
    operation_name: Option<&str>,
    mode: FilterMode,
  ) -> Result<Valid<ExecutableDocument>, RequestError> {
    let mut document = self.document.clone().into_inner();
    let ExecutableDocument {
      ref mut operations,
      ref mut fragments,
      ..
    } = document;
    let operation = operations.get_mut(operation_name)?;
    let selections = &mut operation.selection_set.selections;
    match mode {
      FilterMode::Exclude => Self::filter_exclude(selections, fragments, &self.paths),
      FilterMode::Include => Self::filter_include(selections, fragments, &self.paths),
    }
    // TODO(#1): validate filtered document
    Ok(Valid::assume_valid(document))
  }

  pub(super) fn build_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
      name.into()
    } else {
      format!("{parent}.{name}")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::FilterMode;
  use super::OperationFieldFilter;
  use apollo_compiler::ExecutableDocument;
  use apollo_compiler::Schema;
  use apollo_compiler::validation::Valid;
  use std::fs::read_to_string;
  use std::sync::LazyLock;

  static SCHEMA: LazyLock<Valid<Schema>> = LazyLock::new(|| {
    let sdl = include_str!("../schema.graphql");
    Schema::parse_and_validate(sdl, "schema.graphql").expect("schema should be valid")
  });

  fn extract_mock_paths(content: &str) -> Vec<String> {
    content
      .lines()
      .find(|line| line.starts_with("# mock_paths:"))
      .map(|line| {
        let json = line.trim_start_matches("# mock_paths:").trim();
        serde_json::from_str::<Vec<String>>(json).expect("mock_paths should be valid JSON array")
      })
      .unwrap_or_default()
  }

  #[test]
  fn exclude_fields() {
    insta::glob!("../snapshots", "operation_field_filter/[!e]*/*.graphql", |path| {
      let content = read_to_string(path).expect("should read query file");
      let mock_paths = extract_mock_paths(&content);
      let document = ExecutableDocument::parse_and_validate(&SCHEMA, &content, "query.graphql").expect("should parse");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(
          OperationFieldFilter::new(&document, &mock_paths)
          .filter(None, FilterMode::Exclude)
          .expect("filtering should succeed")
          .to_string()
        );
      });
    });
  }

  #[test]
  fn include_fields() {
    insta::glob!("../snapshots", "operation_field_filter/[!e]*/*.graphql", |path| {
      let content = read_to_string(path).expect("should read query file");
      let mock_paths = extract_mock_paths(&content);
      let document = ExecutableDocument::parse_and_validate(&SCHEMA, &content, "query.graphql").expect("should parse");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(
          OperationFieldFilter::new(&document, &mock_paths)
            .filter(None, FilterMode::Include)
            .expect("filtering should succeed")
            .to_string()
        );
      });
    });
  }

  #[test]
  fn error_handling() {
    insta::glob!(
      "../snapshots",
      "operation_field_filter/error_handling/**/*.graphql",
      |path| {
        let content = read_to_string(path).expect("should read query file");
        let mock_paths = extract_mock_paths(&content);
        let operation_name = content
          .lines()
          .find(|line| line.starts_with("# filter_operation_name:"))
          .map(|line| line.trim_start_matches("# filter_operation_name:").trim().to_string())
          .expect("error_handling test files must have a # filter_operation_name: comment");

        let document =
          ExecutableDocument::parse_and_validate(&SCHEMA, &content, "query.graphql").expect("should parse");

        let err = OperationFieldFilter::new(&document, &mock_paths)
          .filter(Some(&operation_name), FilterMode::Exclude)
          .expect_err("filtering should fail");

        insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
          insta::assert_snapshot!(format!("{err:?}"));
        });
      }
    );
  }

  #[test]
  fn quick_test() {
    let content = r#"
      query GetProducts {
        topProducts(first: 5) {
          __typename
          reviews @mock(hint: "positive reviews from verified buyers") {
            body
          }
        }
      }"#;
    let mock_paths = vec!["topProducts.reviews".to_string()];
    let document = ExecutableDocument::parse_and_validate(&SCHEMA, content, "query.graphql").expect("should parse");

    let router = OperationFieldFilter::new(&document, &mock_paths)
      .filter(None, FilterMode::Include)
      .expect("filtering should succeed")
      .to_string();
    let gen_ai = OperationFieldFilter::new(&document, &mock_paths)
      .filter(None, FilterMode::Exclude)
      .expect("filtering should succeed")
      .to_string();

    println!("{router}");
    println!("{gen_ai}");
  }
}
