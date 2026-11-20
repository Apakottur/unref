//! The findings of a project, and the unused definitions that only other unused definitions reach.

use std::path::Path;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::definitions::Kind;
use crate::ignores::apply_ignore_comments;
use crate::models::Finding;
use crate::transitive::transitive;

use super::{DefinitionId, Graph, Project};

/// Find the unused definitions that only other unused definitions reach.
///
/// An unused function reaches what its code uses, as it would if it were used. An unused class reaches
/// what is defined in it, and an unused function what is defined in it, since deleting them deletes
/// those too. An ignored definition is used, so it reaches nothing here.
pub(super) fn transitive_definitions(
    project: &Project,
    graph: &Graph,
    unused: &[DefinitionId],
) -> FxHashSet<DefinitionId> {
    let position: FxHashMap<DefinitionId, usize> = unused
        .iter()
        .enumerate()
        .map(|(index, &id)| (id, index))
        .collect();
    let mut successors: Vec<Vec<usize>> = unused
        .iter()
        .map(|&id| {
            graph
                .reached(id)
                .into_iter()
                .filter_map(|other| position.get(&other).copied())
                .collect()
        })
        .collect();

    let mut classes: FxHashMap<(usize, String), usize> = FxHashMap::default();
    // By the line of their name, which is the owner of the code in them.
    let mut functions: FxHashMap<(usize, u32), usize> = FxHashMap::default();
    for (index, &id) in unused.iter().enumerate() {
        let (file, definition) = (&project.definition_files[id], &project.definitions[id]);
        if definition.kind == Kind::Class {
            classes.insert((*file, definition.qualified_name()), index);
        } else if definition.kind.is_function() {
            functions.insert((*file, definition.line), index);
        }
    }
    for (index, &id) in unused.iter().enumerate() {
        let (file, definition) = (&project.definition_files[id], &project.definitions[id]);
        // A definition in a class is in the class first, even if the class is in a function.
        let container = match (&definition.class, definition.owner) {
            (Some(class), _) => classes.get(&(*file, class.clone())),
            (None, Some(function)) => functions.get(&(*file, function)),
            (None, None) => None,
        };
        if let Some(&container) = container {
            successors[container].push(index);
        }
    }

    transitive(&successors)
        .into_iter()
        .enumerate()
        .filter(|(_, reached)| *reached)
        .map(|(index, _)| unused[index])
        .collect()
}

/// Make the findings of a project.
///
/// Returns a finding for each unused definition, for each problem with an ignore comment, and for each
/// file that does not parse, sorted by file, line and column.
pub(super) fn findings(
    project: &Project,
    discovered: &[Arc<Path>],
    syntax_errors: &FxHashMap<Arc<Path>, Finding>,
    used: &[bool],
    needed: &[DefinitionId],
    is_transitive: &FxHashSet<DefinitionId>,
) -> Vec<Finding> {
    let file_of: FxHashMap<&Arc<Path>, usize> = project
        .files
        .iter()
        .enumerate()
        .map(|(index, path)| (path, index))
        .collect();
    let mut all = Vec::new();
    for path in discovered {
        if let Some(finding) = syntax_errors.get(path) {
            all.push(finding.clone());
            continue;
        }
        // A file that is read and parsed is in the project.
        let file = file_of[path];
        let mut file_findings: Vec<Finding> = project
            .ids_of_file(file)
            .filter(|id| !used[*id])
            .map(|id| {
                let definition = &project.definitions[id];
                Finding {
                    path: definition.path.to_path_buf(),
                    line: definition.line,
                    column: definition.column,
                    code: definition.kind.code(),
                    name: definition.name.clone(),
                    transitive: is_transitive.contains(&id),
                }
            })
            .collect();

        // A code of an ignore comment that ignores nothing is a finding itself.
        let ignored: Vec<(u32, &str)> = needed
            .iter()
            .filter(|id| project.definition_files[**id] == file)
            .map(|id| {
                let definition = &project.definitions[*id];
                (definition.line, definition.kind.code().name())
            })
            .collect();
        let file_facts = &project.facts[file];
        file_findings.extend(apply_ignore_comments(
            path,
            &file_facts.ignore_comments,
            &file_facts.ignore_findings,
            &ignored,
        ));
        file_findings.sort_by_key(|finding| (finding.line, finding.column));
        all.extend(file_findings);
    }
    all
}
