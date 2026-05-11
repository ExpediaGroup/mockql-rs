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

use super::OperationFieldFilter;
use apollo_compiler::Name;
use apollo_compiler::executable::FragmentMap;
use apollo_compiler::executable::FragmentSpread;
use apollo_compiler::executable::Selection;
use std::collections::HashSet;

impl OperationFieldFilter<'_> {
  /// Keeps only fields whose paths match (or whose subtree contains a match).
  pub(super) fn filter_include(selections: &mut Vec<Selection>, fragments: &mut FragmentMap, paths: &HashSet<String>) {
    let mut protected = HashSet::new();
    Self::collect_protected_fragments(selections, fragments, paths, "", &mut protected);
    Self::filter_include_selections(selections, fragments, paths, "", &mut protected);
    fragments.retain(|_, fragment| !fragment.selection_set.selections.is_empty());
  }

  fn filter_include_selections(
    selections: &mut Vec<Selection>,
    fragments: &mut FragmentMap,
    paths: &HashSet<String>,
    current_path: &str,
    protected_fragments: &mut HashSet<Name>,
  ) {
    selections.retain_mut(|selection| match selection {
      Selection::Field(field) => {
        let field_path = Self::build_path(current_path, field.response_key());
        if paths.contains(&field_path) {
          Self::collect_fragment_names(&field.selection_set.selections, fragments, protected_fragments);
          return true;
        }
        Self::filter_include_selections(
          &mut field.make_mut().selection_set.selections,
          fragments,
          paths,
          &field_path,
          protected_fragments,
        );
        !field.selection_set.selections.is_empty()
      }
      Selection::InlineFragment(inline) => {
        Self::filter_include_selections(
          &mut inline.make_mut().selection_set.selections,
          fragments,
          paths,
          current_path,
          protected_fragments,
        );
        !inline.selection_set.selections.is_empty()
      }
      Selection::FragmentSpread(spread) => {
        if protected_fragments.contains(&spread.fragment_name) {
          let prefix = format!("{current_path}.");
          if !paths.iter().any(|p| p.starts_with(&prefix)) {
            return false;
          }

          let original_name = spread.fragment_name.clone();
          let mut clone_name = Name::new_unchecked(&format!("{original_name}__0"));
          let mut i = 1;
          while fragments.contains_key(clone_name.as_str()) {
            clone_name = Name::new_unchecked(&format!("{original_name}__{i}"));
            i += 1;
          }

          if let Some(original) = fragments.get(&original_name).cloned() {
            let mut cloned = original;
            cloned.make_mut().name = clone_name.clone();
            fragments.insert(clone_name.clone(), cloned);
          }

          spread.make_mut().fragment_name = clone_name;

          Self::filter_fragment_include(spread, fragments, paths, current_path, protected_fragments);

          return fragments
            .get(&spread.fragment_name)
            .is_some_and(|f| !f.selection_set.selections.is_empty());
        }

        Self::filter_fragment_include(spread, fragments, paths, current_path, protected_fragments);
        fragments
          .get(&spread.fragment_name)
          .is_none_or(|f| !f.selection_set.selections.is_empty())
      }
    });
  }

  /// Walks selections recursively to discover fragments referenced by matched field's subtrees.
  fn collect_protected_fragments(
    selections: &[Selection],
    fragments: &FragmentMap,
    paths: &HashSet<String>,
    current_path: &str,
    protected: &mut HashSet<Name>,
  ) {
    for selection in selections {
      match selection {
        Selection::Field(field) => {
          let field_path = Self::build_path(current_path, field.response_key());
          if paths.contains(&field_path) {
            Self::collect_fragment_names(&field.selection_set.selections, fragments, protected);
          } else {
            Self::collect_protected_fragments(
              &field.selection_set.selections,
              fragments,
              paths,
              &field_path,
              protected,
            );
          }
        }
        Selection::InlineFragment(inline) => {
          Self::collect_protected_fragments(
            &inline.selection_set.selections,
            fragments,
            paths,
            current_path,
            protected,
          );
        }
        Selection::FragmentSpread(spread) => {
          if let Some(fragment) = fragments.get(&spread.fragment_name) {
            Self::collect_protected_fragments(
              &fragment.selection_set.selections,
              fragments,
              paths,
              current_path,
              protected,
            );
          }
        }
      }
    }
  }

  /// Recursively collects all fragment names referenced in a selection set,
  /// including fragments referenced transitively by other fragments.
  fn collect_fragment_names(selections: &[Selection], fragments: &FragmentMap, collected: &mut HashSet<Name>) {
    for selection in selections {
      match selection {
        Selection::Field(field) => {
          Self::collect_fragment_names(&field.selection_set.selections, fragments, collected);
        }
        Selection::InlineFragment(inline) => {
          Self::collect_fragment_names(&inline.selection_set.selections, fragments, collected);
        }
        Selection::FragmentSpread(spread) => {
          if collected.insert(spread.fragment_name.clone())
            && let Some(fragment) = fragments.get(&spread.fragment_name)
          {
            Self::collect_fragment_names(&fragment.selection_set.selections, fragments, collected);
          }
        }
      }
    }
  }

  /// Filters a fragment's selections for include mode. Uses take–modify–put
  /// back because the recursive filter call needs `&mut fragments`, preventing
  /// a simultaneous borrow into a single fragment's fields.
  fn filter_fragment_include(
    spread: &FragmentSpread,
    fragments: &mut FragmentMap,
    paths: &HashSet<String>,
    current_path: &str,
    protected_fragments: &mut HashSet<Name>,
  ) {
    let fragment_name = spread.fragment_name.clone();
    let taken = fragments
      .get_mut(&fragment_name)
      .map(|f| std::mem::take(&mut f.make_mut().selection_set.selections));
    if let Some(mut frag_selections) = taken {
      Self::filter_include_selections(
        &mut frag_selections,
        fragments,
        paths,
        current_path,
        protected_fragments,
      );
      // The entry was just accessed above and filtering never removes fragment entries
      fragments
        .get_mut(&fragment_name)
        .expect("fragment entry was not removed during filtering")
        .make_mut()
        .selection_set
        .selections = frag_selections;
    }
  }
}
