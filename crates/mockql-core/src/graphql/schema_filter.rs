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

//! Schema filtering helpers for producing prompt-local schemas.

use super::types::SchemaTypes;
use apollo_compiler::ExecutableDocument;
use apollo_compiler::Name;
use apollo_compiler::Schema;
use apollo_compiler::ast::FieldDefinition;
use apollo_compiler::ast::NamedType;
use apollo_compiler::collections::IndexMap;
use apollo_compiler::collections::IndexSet;
use apollo_compiler::executable::Field;
use apollo_compiler::executable::FragmentSpread;
use apollo_compiler::executable::InlineFragment;
use apollo_compiler::executable::Selection;
use apollo_compiler::executable::SelectionSet;
use apollo_compiler::schema::Component;
use apollo_compiler::schema::ExtendedType;
use apollo_compiler::validation::Valid;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

/// Filters a schema down to the types and fields referenced by an operation.
#[derive(Debug)]
pub struct SchemaFilter {
  schema: Arc<Valid<Schema>>,
}

/// Errors raised while collecting the schema subset needed by an operation.
#[derive(Debug, thiserror::Error)]
pub enum SchemaFilterError {
  /// The operation could not be parsed against the active schema.
  #[error("failed to parse operation: {0:?}")]
  ParseError(Vec<String>),
  /// The requested operation name was not present in the document.
  #[error("operation not found: {0}")]
  OperationNotFound(String),
}

impl SchemaFilter {
  /// Creates a new schema filter for the supplied validated schema.
  pub fn new(schema: Arc<Valid<Schema>>) -> Self {
    Self { schema }
  }

  /// Produces the subset of schema types and fields required by an operation.
  pub fn filter(&self, query: &str, operation_name: Option<&str>) -> Result<SchemaTypes, SchemaFilterError> {
    let ReferencedTypes { required, fields } = self.get_referenced_types(query, operation_name)?;
    let required_types: HashSet<String> = required.iter().cloned().collect();
    let types = required
      .iter()
      .filter_map(|type_name| {
        self.schema.types.get(type_name.as_str()).map(|extended_type| {
          let filtered = self.filter_type(extended_type, fields.get(type_name), &required_types);
          (extended_type.name().clone(), filtered)
        })
      })
      .collect::<IndexMap<NamedType, ExtendedType>>();
    Ok(SchemaTypes::new(types))
  }

  fn get_referenced_types(
    &self,
    query: &str,
    operation_name: Option<&str>,
  ) -> Result<ReferencedTypes, SchemaFilterError> {
    let document = ExecutableDocument::parse(&self.schema, query, "query.graphql")
      .map_err(|e| SchemaFilterError::ParseError(e.errors.iter().map(|d| d.error.to_string()).collect()))?;

    let operation = document
      .operations
      .get(operation_name)
      .map_err(|e| SchemaFilterError::OperationNotFound(format!("{e:?}")))?;

    let mut traversal = SchemaFilterTraversal::new(&self.schema, &document);
    traversal.collect_from_selection_set(&operation.selection_set, operation.selection_set.ty.as_str());

    for variable in &operation.variables {
      traversal
        .required_types
        .insert(variable.ty.inner_named_type().to_string());
    }

    traversal.collect_transitive_dependencies();

    Ok(ReferencedTypes {
      fields: traversal.referenced,
      required: {
        let mut required: Vec<String> = traversal.required_types.into_iter().collect();
        required.sort();
        required
      },
    })
  }

  fn filter_type(
    &self,
    extended_type: &ExtendedType,
    referenced_fields: Option<&HashSet<String>>,
    required_types: &HashSet<String>,
  ) -> ExtendedType {
    let mut filtered_type = extended_type.clone();
    match &mut filtered_type {
      ExtendedType::Object(object) => {
        object.make_mut().fields = self.filter_fields(referenced_fields, &object.fields);
      }
      ExtendedType::Interface(interface) => {
        interface.make_mut().fields = self.filter_fields(referenced_fields, &interface.fields);
      }
      ExtendedType::Union(union_type) => {
        let filtered_members: IndexSet<_> = union_type
          .members
          .iter()
          .filter(|member| required_types.contains(member.as_str()))
          .cloned()
          .collect();

        union_type.make_mut().members = if filtered_members.is_empty() {
          union_type.members.iter().take(1).cloned().collect()
        } else {
          filtered_members
        };
      }
      _ => {}
    }
    filtered_type
  }

  fn filter_fields(
    &self,
    referenced_fields: Option<&HashSet<String>>,
    all_fields: &IndexMap<Name, Component<FieldDefinition>>,
  ) -> IndexMap<Name, Component<FieldDefinition>> {
    let Some(fields) = referenced_fields.filter(|f| !f.is_empty()) else {
      return all_fields.clone();
    };

    let filtered: IndexMap<_, _> = all_fields
      .iter()
      .filter(|(name, _)| fields.contains(name.as_str()))
      .map(|(name, component)| (name.clone(), component.clone()))
      .collect();

    if filtered.is_empty() {
      all_fields.clone()
    } else {
      filtered
    }
  }
}

struct ReferencedTypes {
  required: Vec<String>,
  fields: HashMap<String, HashSet<String>>,
}

struct SchemaFilterTraversal<'a> {
  schema: &'a Valid<Schema>,
  document: &'a ExecutableDocument,
  referenced: HashMap<String, HashSet<String>>,
  required_types: HashSet<String>,
  visited_fragments: HashSet<Name>,
}

impl<'a> SchemaFilterTraversal<'a> {
  fn new(schema: &'a Valid<Schema>, document: &'a ExecutableDocument) -> Self {
    Self {
      schema,
      document,
      referenced: HashMap::new(),
      required_types: HashSet::new(),
      visited_fragments: HashSet::new(),
    }
  }

  fn collect_from_selection_set(&mut self, selection_set: &SelectionSet, parent_type_name: &str) {
    self.required_types.insert(parent_type_name.to_string());

    for selection in &selection_set.selections {
      match selection {
        Selection::Field(field) => self.collect_field(field, parent_type_name),
        Selection::FragmentSpread(spread) => self.collect_fragment_spread(spread),
        Selection::InlineFragment(inline) => self.collect_inline_fragment(inline, parent_type_name),
      }
    }
  }

  fn collect_field(&mut self, field: &Field, parent_type_name: &str) {
    let field_name = field.name.as_str();

    self
      .referenced
      .entry(parent_type_name.to_string())
      .or_default()
      .insert(field_name.to_string());

    // Propagate field references to implemented interfaces.
    if let Some(ExtendedType::Object(object)) = self.schema.types.get(parent_type_name) {
      for iface_name in &object.implements_interfaces {
        if self.schema.type_field(iface_name.as_str(), field_name).is_ok() {
          self
            .referenced
            .entry(iface_name.to_string())
            .or_default()
            .insert(field_name.to_string());
        }
      }
    }

    let Ok(definition) = self.schema.type_field(parent_type_name, field_name) else {
      return;
    };

    let field_type_name = definition.ty.inner_named_type().to_string();
    self.required_types.insert(field_type_name.clone());

    // Include argument types
    for arg in &definition.arguments {
      self.required_types.insert(arg.ty.inner_named_type().to_string());
    }

    // Recurse into nested selections
    if !field.selection_set.selections.is_empty() {
      self.collect_from_selection_set(&field.selection_set, &field_type_name);
    }
  }

  fn collect_fragment_spread(&mut self, spread: &FragmentSpread) {
    if self.visited_fragments.contains(&spread.fragment_name) {
      return;
    }
    self.visited_fragments.insert(spread.fragment_name.clone());

    let Some(fragment) = self.document.fragments.get(&spread.fragment_name) else {
      return;
    };

    let type_condition = fragment.type_condition().to_string();
    self.required_types.insert(type_condition.clone());
    self.collect_from_selection_set(&fragment.selection_set, &type_condition);
  }

  fn collect_inline_fragment(&mut self, inline: &InlineFragment, parent_type_name: &str) {
    let type_name = inline
      .type_condition
      .as_ref()
      .map_or(parent_type_name, |tc| tc.as_str());

    let type_name = type_name.to_string();
    self.required_types.insert(type_name.clone());
    self.collect_from_selection_set(&inline.selection_set, &type_name);
  }

  /// Transitively collects all type dependencies (interfaces, unions, input types, etc.)
  /// **Interfaces**: `query { me { name } }` where `me` returns `User`.
  /// If `User` implements `Node`, the `Node` interface must be included.
  ///
  /// **Unions**: `query { search { ... on Product { name } } }` where `search` returns `SearchResult`.
  /// Only union member types that are actually referenced (for example, via inline fragments like `... on Product`)
  /// are included. If none are referenced, one member is retained as a valid `__typename` candidate.
  fn collect_transitive_dependencies(&mut self) {
    let mut to_process: Vec<String> = self.required_types.iter().cloned().collect();
    let implementers = self.schema.implementers_map();

    while let Some(type_name) = to_process.pop() {
      let Some(type_def) = self.schema.types.get(type_name.as_str()) else {
        continue;
      };

      match type_def {
        ExtendedType::Object(obj) => {
          for iface in &obj.implements_interfaces {
            self.add_type_if_new(iface, &mut to_process);
          }
        }
        ExtendedType::Interface(interface_type) => {
          for iface in &interface_type.implements_interfaces {
            self.add_type_if_new(iface, &mut to_process);
          }
          if let Some(possible_types) = implementers.get(type_name.as_str())
            && !possible_types
              .objects
              .iter()
              .any(|name| self.required_types.contains(name.as_str()))
            && let Some(name) = possible_types.objects.iter().next()
          {
            self.add_type_if_new(name, &mut to_process);
          }
        }
        ExtendedType::Union(union_type) => {
          if !union_type
            .members
            .iter()
            .any(|name| self.required_types.contains(name.as_str()))
            && let Some(name) = union_type.members.iter().next()
          {
            self.add_type_if_new(name, &mut to_process);
          }
        }
        ExtendedType::InputObject(input) => {
          for field in input.fields.values() {
            self.add_type_if_new(field.ty.inner_named_type().as_str(), &mut to_process);
          }
        }
        _ => {}
      }
    }
  }

  fn add_type_if_new(&mut self, type_name: &str, to_process: &mut Vec<String>) {
    if !self.required_types.contains(type_name) {
      let owned_type_name = type_name.to_string();
      to_process.push(owned_type_name.clone());
      self.required_types.insert(owned_type_name);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::read_to_string;
  use std::sync::LazyLock;

  static SCHEMA_FILTER: LazyLock<SchemaFilter> = LazyLock::new(|| {
    let sdl = include_str!("schema.graphql");
    let schema = Schema::parse_and_validate(sdl, "schema.graphql").expect("schema should be valid");
    SchemaFilter::new(Arc::new(schema))
  });

  #[test]
  fn filter_produces_expected_named_types() {
    let content = r"
      query {              # Query
        me {               # User (implements Node)
          name             # String
          reviews {        # Review (implements Node)
            body           # String
            product {      # Product (implements Node & Reviewable)
              name         # String
            }
          }
        }
      }
      ";
    let filtered = SCHEMA_FILTER.filter(content, None).expect("filter should succeed");
    let type_names: Vec<String> = filtered.0.keys().map(std::string::ToString::to_string).collect();
    assert_eq!(
      vec!["Node", "Product", "Query", "Review", "Reviewable", "String", "User"],
      type_names
    );
  }

  #[test]
  fn filter_produces_expected_schema_for_operations() {
    insta::glob!("snapshots/schema_filter/**/*.graphql", |path| {
      let content = read_to_string(path).expect("should read query file");
      let snapshot_path = path.parent().unwrap();
      let operation_name = content
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("# operation_name:"))
        .map(|value| value.trim().to_string());
      insta::with_settings!({snapshot_path => snapshot_path}, {
        insta::assert_snapshot!(
          SCHEMA_FILTER
            .filter(&content, operation_name.as_deref())
            .expect("filter should succeed")
        );
      });
    });
  }

  #[test]
  fn filter_returns_parse_error_for_invalid_query() {
    let invalid_query = "this is not valid graphql {{{";
    let result = SCHEMA_FILTER.filter(invalid_query, None);
    assert!(matches!(result, Err(SchemaFilterError::ParseError(_))));
  }
}
