//! Native's registry list beside the config types' loader paths: found by both, engine only, and
//! each config type that no registry names, with its reason.

use super::projection::registry_types;
use crate::ledger::Ledger;
use pdx_native::{Answer, Completeness, Gap, Registry};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Directories of the format families that Milestone 6 owns.
const DEFERRED_FORMAT_FAMILIES: &[&str] = &[
    "gfx",
    "interface",
    "sound",
    "music",
    "flags",
    "fonts",
    "map/setup_scenarios",
    "prescripted_countries",
];

/// Directories whose config types make up the Milestone 5 gate population, with what is below them.
const GATE_DIRECTORIES: &[&str] = &["common", "events"];

/// How a config type joins Native's registry list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Join {
    /// A Native registry names the type's directory, and no other type loads from it.
    Named,
    /// A Native registry names the directory, but several config types load from it, so the
    /// applicable type is unresolved.
    SeveralTypesInDirectory,
    /// No Native registry names the directory. Native's unnamed-registry count has no names, so it
    /// cannot say whether an engine candidate exists for this type.
    NoNamedRegistry,
    /// No Native registry names the directory, and the directory is `gfx`, `interface`, `sound`,
    /// `music`, `flags`, `fonts`, `map/setup_scenarios` or `prescripted_countries`, or below one of
    /// them: a format family that Milestone 6 owns.
    DeferredFormatFamily,
}

/// One config type and its join to Native's registry list.
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    /// Config type, such as `type[tradition]`.
    pub type_name: String,
    /// Config file that defines the type.
    pub file: String,
    /// Content directory of the type's `path`, without `game/`.
    pub directory: String,
    /// The Native registry that names the directory.
    pub registry: Option<String>,
    /// How the type joins.
    pub join: Join,
    /// Whether the type is in the Milestone 5 gate population: its directory is under `common/` or
    /// `events/`.
    pub gate: bool,
}

/// Native's registry list compared with the config types.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// Report contract version.
    pub format_version: u32,
    /// Source config content identity.
    pub config_sha256: String,
    /// False when unresolved config diagnostics remain; a type behind a diagnostic may be missing.
    pub inventory_complete: bool,
    /// Native's source of the registry list, with its exact build.
    pub native_source: pdx_native::Source,
    /// Whether Native named every candidate inside its search.
    pub native_completeness: Completeness,
    /// Native's gaps in the registry list, including the count of unnamed registries.
    pub native_gaps: Vec<Gap>,
    /// Config types in the gate population.
    pub gate_types: usize,
    /// Config types by join, over every row.
    pub types_by_join: BTreeMap<Join, usize>,
    /// Directories that Native names and at least one config type loads from.
    pub directories_in_both: usize,
    /// Native registries that no config type loads from.
    pub engine_only: Vec<String>,
    /// Every config type with a `path`.
    pub rows: Vec<Row>,
}

/// Compares Native's registry list with the directories of the config types.
pub fn compare(ledger: &Ledger, registries: &Answer<Vec<Registry>>) -> Report {
    let named: BTreeSet<&str> = registries
        .value
        .iter()
        .map(|registry| registry.name.as_str())
        .collect();
    let directories = registry_types(ledger);
    let mut rows = Vec::new();
    for (directory, types) in &directories {
        let registry = named
            .contains(directory.as_str())
            .then(|| directory.clone());
        let join = match (&registry, types.len()) {
            (Some(_), 1) => Join::Named,
            (Some(_), _) => Join::SeveralTypesInDirectory,
            (None, _) if below(directory, DEFERRED_FORMAT_FAMILIES) => Join::DeferredFormatFamily,
            (None, _) => Join::NoNamedRegistry,
        };
        for (type_name, file) in types {
            rows.push(Row {
                type_name: type_name.clone(),
                file: file.clone(),
                directory: directory.clone(),
                registry: registry.clone(),
                join,
                gate: below(directory, GATE_DIRECTORIES),
            });
        }
    }
    let mut types_by_join = BTreeMap::new();
    for row in &rows {
        *types_by_join.entry(row.join).or_default() += 1;
    }
    Report {
        format_version: 1,
        config_sha256: ledger.config_sha256.clone(),
        inventory_complete: ledger.diagnostics.is_empty(),
        native_source: registries.source.clone(),
        native_completeness: registries.completeness,
        native_gaps: registries.gaps.clone(),
        gate_types: rows.iter().filter(|row| row.gate).count(),
        types_by_join,
        directories_in_both: directories
            .keys()
            .filter(|directory| named.contains(directory.as_str()))
            .count(),
        engine_only: named
            .iter()
            .filter(|name| !directories.contains_key(**name))
            .map(|name| (*name).to_owned())
            .collect(),
        rows,
    }
}

/// Whether `directory` is one of `roots` or below one of them.
fn below(directory: &str, roots: &[&str]) -> bool {
    roots.iter().any(|root| {
        directory
            .strip_prefix(root)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}
