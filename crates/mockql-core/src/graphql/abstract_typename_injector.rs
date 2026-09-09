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

//! Prompt-only insertion of `__typename` for abstract GraphQL fields.

use apollo_compiler::ExecutableDocument;
use apollo_compiler::Name;
use apollo_compiler::Schema;
use apollo_compiler::executable::FragmentMap;
use apollo_compiler::executable::Selection;
use apollo_compiler::executable::SelectionSet;
use apollo_compiler::name;
use apollo_compiler::request::RequestError;
use apollo_compiler::schema::ExtendedType;
use apollo_compiler::validation::Valid;
use std::collections::HashSet;

pub(crate) fn add_typename_to_abstract_types(
  schema: &Valid<Schema>,
  document: Valid<ExecutableDocument>,
  operation_name: Option<&str>,
) -> Result<Valid<ExecutableDocument>, RequestError> {
  let mut document = document.into_inner();
  let ExecutableDocument {
    operations, fragments, ..
  } = &mut document;
  let operation = operations.get_mut(operation_name)?;
  inject_selection_set(schema, &mut operation.selection_set, fragments, &mut HashSet::new());

  Ok(Valid::assume_valid(document))
}

fn inject_selection_set(
  schema: &Schema,
  selection_set: &mut SelectionSet,
  fragments: &mut FragmentMap,
  visited_fragments: &mut HashSet<Name>,
) {
  for selection in &mut selection_set.selections {
    match selection {
      Selection::Field(field) => {
        let field = field.make_mut();
        let is_abstract = matches!(
          schema.types.get(field.selection_set.ty.as_str()),
          Some(ExtendedType::Interface(_) | ExtendedType::Union(_))
        );
        let has_typename = field
          .selection_set
          .fields()
          .any(|field| field.response_key() == "__typename");

        if is_abstract
          && !has_typename
          && let Ok(typename) = field.selection_set.new_field(schema, name!("__typename"))
        {
          field.selection_set.push(typename);
        }

        inject_selection_set(schema, &mut field.selection_set, fragments, visited_fragments);
      }
      Selection::InlineFragment(inline) => inject_selection_set(
        schema,
        &mut inline.make_mut().selection_set,
        fragments,
        visited_fragments,
      ),
      Selection::FragmentSpread(spread) => {
        if visited_fragments.insert(spread.fragment_name.clone())
          && let Some(mut fragment) = fragments.get(&spread.fragment_name).cloned()
        {
          inject_selection_set(
            schema,
            &mut fragment.make_mut().selection_set,
            fragments,
            visited_fragments,
          );
          fragments.insert(spread.fragment_name.clone(), fragment);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::read_to_string;
  use std::sync::LazyLock;

  static SCHEMA: LazyLock<Valid<Schema>> = LazyLock::new(|| {
    Schema::parse_and_validate(include_str!("schema.graphql"), "schema.graphql").expect("schema should be valid")
  });

  #[test]
  fn should_add_typename_to_abstract_types() {
    insta::glob!("snapshots/abstract_typename_injector/**/*.graphql", |path| {
      let content = read_to_string(path).expect("should read query file");
      let operation_name = content
        .lines()
        .find_map(|line| line.strip_prefix("# operation_name:"))
        .map(str::trim);
      let document =
        ExecutableDocument::parse_and_validate(&SCHEMA, &content, "query.graphql").expect("query should be valid");

      insta::with_settings!({snapshot_path => path.parent().unwrap(), omit_expression => true}, {
        insta::assert_snapshot!(
          add_typename_to_abstract_types(&SCHEMA, document, operation_name)
            .expect("operation should exist")
            .to_string()
        );
      });
    });
  }
}
