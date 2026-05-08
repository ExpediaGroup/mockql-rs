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
use apollo_compiler::executable::FragmentMap;
use apollo_compiler::executable::FragmentSpread;
use apollo_compiler::executable::Selection;
use std::collections::HashSet;

impl OperationFieldFilter<'_> {
  /// Removes fields whose paths match, keeping everything else.
  pub(super) fn filter_exclude(selections: &mut Vec<Selection>, fragments: &mut FragmentMap, paths: &HashSet<String>) {
    Self::filter_exclude_selections(selections, fragments, paths, "");
  }

  fn filter_exclude_selections(
    selections: &mut Vec<Selection>,
    fragments: &mut FragmentMap,
    paths: &HashSet<String>,
    current_path: &str,
  ) {
    selections.retain_mut(|selection| match selection {
      Selection::Field(field) => {
        let field_path = Self::build_path(current_path, field.response_key());
        if paths.contains(&field_path) {
          return false;
        }
        Self::filter_exclude_selections(
          &mut field.make_mut().selection_set.selections,
          fragments,
          paths,
          &field_path,
        );
        true
      }
      Selection::InlineFragment(inline) => {
        Self::filter_exclude_selections(
          &mut inline.make_mut().selection_set.selections,
          fragments,
          paths,
          current_path,
        );
        true
      }
      Selection::FragmentSpread(spread) => {
        Self::filter_fragment_exclude(spread, fragments, paths, current_path);
        true
      }
    });
  }

  /// Filters a fragment's selections for exclude mode. Uses take–modify–put
  /// back because the recursive filter call needs `&mut fragments`, preventing
  /// a simultaneous borrow into a single fragment's fields.
  fn filter_fragment_exclude(
    spread: &FragmentSpread,
    fragments: &mut FragmentMap,
    paths: &HashSet<String>,
    current_path: &str,
  ) {
    let fragment_name = spread.fragment_name.clone();
    let taken = fragments
      .get_mut(&fragment_name)
      .map(|f| std::mem::take(&mut f.make_mut().selection_set.selections));
    if let Some(mut frag_selections) = taken {
      Self::filter_exclude_selections(&mut frag_selections, fragments, paths, current_path);
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
