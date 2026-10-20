//! The config's own facts about its types, read once from a ledger: the types that each file
//! defines, each type's `path` lines, the selector lines of each type and subtype
//! (`type_key_filter`, `skip_root_key`, `base_type`, `path_file`) and the `subtype[…]` syntax.
//! They do not depend on a snapshot: the projection resolves each selector against the Native
//! answers of the snapshot it projects.

use crate::ledger::{Claim, Ledger};
use std::collections::{BTreeMap, BTreeSet};

/// The config types that load from each content directory, as (type, file).
pub(crate) type RegistryTypes = BTreeMap<String, BTreeSet<(String, String)>>;
/// Config subtypes whose whole selector is a root key, by (file, type, root key).
pub(crate) type KeySubtypes<'a> = BTreeMap<(&'a str, &'a str, &'a str), BTreeSet<&'a str>>;
/// The config types that each file defines, including files that define none.
pub(crate) type FileTypes = BTreeMap<String, BTreeSet<String>>;

/// The config's facts about its types.
pub(crate) struct ConfigTypes<'a> {
    /// The types that each file defines, including files that define none.
    pub(crate) file_types: FileTypes,
    /// Each type's `path` lines as the config writes them, quoted, by (file, type).
    pub(crate) paths: BTreeMap<(&'a str, &'a str), Vec<&'a str>>,
    /// The types of each content directory that a type's own file defines.
    pub(crate) registry_types: RegistryTypes,
    /// The selector lines of each type, by (file, type).
    selector_lines: BTreeMap<(&'a str, &'a str), SelectorLines<'a>>,
    /// Config subtypes whose whole selector is a root key.
    pub(crate) key_subtypes: KeySubtypes<'a>,
    /// The IDs of the claims under a subtype that a root key selects.
    pub(crate) root_key_subtype_claims: BTreeSet<&'a str>,
}

impl<'a> ConfigTypes<'a> {
    pub(crate) fn new(ledger: &'a Ledger) -> Self {
        let file_types = file_types(ledger);
        let paths = type_paths(ledger);
        let registry_types = registry_types(&paths, &file_types);

        Self {
            root_key_subtype_claims: root_key_subtype_claims(ledger, &file_types),
            selector_lines: selector_lines(ledger),
            key_subtypes: key_subtypes(ledger),
            file_types,
            paths,
            registry_types,
        }
    }

    /// The config type that `claim` belongs to: claims below `types/type[x]`, and claims below
    /// the definition root `x` in the file that defines `type[x]`.
    pub(crate) fn claim_type(&self, claim: &Claim) -> Option<&str> {
        claim_type(claim, &self.file_types)
    }

    /// The selector lines of `type_name` in `file`; none for a type that writes none.
    pub(crate) fn selector_lines<'s>(
        &'s self,
        file: &'s str,
        type_name: &'s str,
    ) -> &'s SelectorLines<'a> {
        const NO_LINES: &SelectorLines<'_> = &SelectorLines {
            key_filter: None,
            skip_root_key: BTreeMap::new(),
            base_type: None,
            path_file: None,
        };

        let selector_lines: &BTreeMap<(&'s str, &'s str), SelectorLines<'a>> = &self.selector_lines;

        selector_lines.get(&(file, type_name)).unwrap_or(NO_LINES)
    }
}

/// Whether a subject segment or condition is a `subtype[…]`.
pub(crate) fn is_subtype(segment: &str) -> bool {
    segment.starts_with("subtype[")
}

/// The content directory of a config `path`. Two config types write their path without `game/`;
/// the directory is the same.
pub(crate) fn config_directory(path: &str) -> &str {
    let path = path.trim_matches('"');
    path.strip_prefix("game/").unwrap_or(path)
}

/// Config directories that the engine loads under another name, each with the engine's name. The
/// config's `path` stays its claim, so the loader-path comparison shows the difference; the type
/// joins the engine's registry and reads that folder's comments. `lawsuit`: the config writes
/// `game/common/lawsuit_type`, but the executable names only `common/lawsuits`, the game ships that
/// folder, and Native's registry reads the config's three fields (`cost`, `duration`, `effect`).
const RENAMED_DIRECTORIES: &[(&str, &str)] = &[("common/lawsuit_type", "common/lawsuits")];

/// The content directory that a type's config `path` names, as the engine loads it.
pub(crate) fn content_directory(path: &str) -> &str {
    let directory = config_directory(path);

    RENAMED_DIRECTORIES
        .iter()
        .find(|(config, _)| *config == directory)
        .map_or(directory, |(_, engine)| engine)
}

fn claim_type<'t>(claim: &Claim, file_types: &'t FileTypes) -> Option<&'t str> {
    let first = claim.subject.first()?;
    let type_name = match claim.subject.get(1) {
        Some(second) if first == "types" => second.clone(),
        _ => format!("type[{first}]"),
    };

    file_types[&claim.file].get(&type_name).map(String::as_str)
}

fn file_types(ledger: &Ledger) -> FileTypes {
    let mut types: BTreeMap<_, _> = ledger
        .files
        .iter()
        .map(|f| (f.path.clone(), BTreeSet::new()))
        .collect();
    for claim in &ledger.claims {
        if let [types_key, type_name] = claim.subject.as_slice()
            && types_key == "types"
            && claim.property == "type_existence"
        {
            types
                .get_mut(&claim.file)
                .expect("claim file")
                .insert(type_name.clone());
        }
    }
    types
}

/// Each type's `path` lines, in ledger order, by (file, type).
fn type_paths(ledger: &Ledger) -> BTreeMap<(&str, &str), Vec<&str>> {
    let mut paths: BTreeMap<_, Vec<_>> = BTreeMap::new();

    for claim in &ledger.claims {
        if let [types, type_name, path] = claim.subject.as_slice()
            && types == "types"
            && path == "path"
            && claim.property == "loader_path"
        {
            paths
                .entry((claim.file.as_str(), type_name.as_str()))
                .or_default()
                .push(claim.config_answer.as_str());
        }
    }

    paths
}

fn registry_types(
    paths: &BTreeMap<(&str, &str), Vec<&str>>,
    file_types: &FileTypes,
) -> RegistryTypes {
    let mut candidates = RegistryTypes::new();

    for ((file, type_name), paths) in paths {
        if !file_types[*file].contains(*type_name) {
            continue;
        }

        for path in paths {
            candidates
                .entry(content_directory(path).into())
                .or_default()
                .insert(((*type_name).to_owned(), (*file).to_owned()));
        }
    }

    candidates
}

/// A `## type_key_filter` annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyFilter<'a> {
    /// `type_key_filter = KEY`: definitions whose key is `KEY`.
    Equals(&'a str),
    /// `type_key_filter <> KEY`: definitions whose key is not `KEY`.
    Excludes(&'a str),
    /// Any other form, such as a `{ … }` list or an empty key, as written after `type_key_filter`.
    Other(&'a str),
}

/// The filter that a `## type_key_filter` annotation writes. `config_answer` drops a `<>`
/// operator, so this reads the source text.
fn key_filter(claim: &Claim) -> Option<KeyFilter<'_>> {
    let rest = claim
        .source_text
        .trim_start_matches('#')
        .trim()
        .strip_prefix("type_key_filter")?
        .trim_start();
    let (excludes, key) = match rest.strip_prefix("<>") {
        Some(key) => (true, key.trim()),
        None => (false, rest.strip_prefix('=')?.trim()),
    };
    let one_key = !key.is_empty() && !key.contains(char::is_whitespace) && !key.contains('{');

    Some(match (one_key, excludes) {
        (false, _) => KeyFilter::Other(rest),
        (true, true) => KeyFilter::Excludes(key),
        (true, false) => KeyFilter::Equals(key),
    })
}

/// The lines of a type's definition that select its definitions in the directory.
#[derive(Debug, Default)]
pub(crate) struct SelectorLines<'a> {
    key_filter: Option<KeyFilter<'a>>,
    /// `skip_root_key` keys in order: one key at 0, or the items of a `{ … }` list.
    skip_root_key: BTreeMap<usize, &'a str>,
    base_type: Option<&'a str>,
    /// The one file of the type's directory that the type loads from.
    pub(crate) path_file: Option<&'a str>,
}

impl SelectorLines<'_> {
    /// Whether these lines leave the type among the directory's root definitions.
    pub(crate) fn selects_root(&self) -> bool {
        self.skip_root_key.is_empty()
            && matches!(self.key_filter, None | Some(KeyFilter::Excludes(_)))
    }

    /// The root key that alone selects the type's definitions.
    pub(crate) fn root_key(&self) -> Option<&str> {
        let Some(KeyFilter::Equals(key)) = self.key_filter else {
            return None;
        };

        self.skip_root_key.is_empty().then_some(key)
    }

    /// The field path of a nested definition below the `root` type's definitions.
    pub(crate) fn nested_path(&self, root: Option<&str>) -> Option<String> {
        let root = root?.strip_prefix("type[")?.strip_suffix(']')?;
        let Some(KeyFilter::Equals(key)) = self.key_filter else {
            return None;
        };
        let mut skipped = self.skip_root_key.values().copied();
        let base = self.base_type?.split('.').next();

        if skipped.next() != Some("any") || base != Some(root) {
            return None;
        }

        Some(skipped.chain([key]).collect::<Vec<_>>().join("/"))
    }

    /// The gap reason for a type whose selector no Native answer identifies, naming its lines.
    pub(crate) fn unresolved_reason(&self) -> String {
        let mut lines = Vec::new();

        match self.key_filter {
            Some(KeyFilter::Equals(key)) => lines.push(format!("type_key_filter = {key}")),
            Some(KeyFilter::Excludes(key)) => lines.push(format!("type_key_filter <> {key}")),
            Some(KeyFilter::Other(text)) => lines.push(format!("type_key_filter {text}")),
            None => {}
        }

        match self
            .skip_root_key
            .values()
            .copied()
            .collect::<Vec<_>>()
            .as_slice()
        {
            [] => {}
            [key] => lines.push(format!("skip_root_key = {key}")),
            keys => lines.push(format!("skip_root_key = {{ {} }}", keys.join(" "))),
        }

        if let Some(base) = self.base_type {
            lines.push(format!("base_type = {base}"));
        }

        if lines.is_empty() {
            return "The type has no `type_key_filter = KEY` or `skip_root_key`, and neither has \
                    another type in its directory; the applicable type is unresolved"
                .into();
        }

        format!(
            "The type is selected by `{}`; no Native answer identifies it",
            lines.join("`, `")
        )
    }
}

/// The selector lines of each config type, by (file, type).
fn selector_lines(ledger: &Ledger) -> BTreeMap<(&str, &str), SelectorLines<'_>> {
    let mut selectors: BTreeMap<_, SelectorLines<'_>> = BTreeMap::new();

    for claim in &ledger.claims {
        let [types, type_name, line @ ..] = claim.subject.as_slice() else {
            continue;
        };

        if types != "types" {
            continue;
        }

        let lines = selectors
            .entry((claim.file.as_str(), type_name.as_str()))
            .or_default();
        let answer = claim.config_answer.trim_matches('"');

        match line {
            [annotation] if annotation == "$annotation:type_key_filter" => {
                lines.key_filter = key_filter(claim);
            }
            [key] if key == "skip_root_key" && answer != "block" => {
                lines.skip_root_key.insert(0, answer);
            }
            [key, item] if key == "skip_root_key" => {
                if let Some(position) = item.strip_prefix("$item:").and_then(|n| n.parse().ok()) {
                    lines.skip_root_key.insert(position, answer);
                }
            }
            [key] if key == "base_type" => lines.base_type = Some(answer),
            [key] if key == "path_file" => lines.path_file = Some(answer),
            _ => {}
        }
    }

    selectors
}

/// Config subtypes that `## type_key_filter = KEY` selects and whose definition body is empty, so
/// the root key is their whole selector. A subtype whose body holds any key is left out.
fn key_subtypes(ledger: &Ledger) -> KeySubtypes<'_> {
    let with_body = ledger
        .claims
        .iter()
        .filter_map(|claim| match claim.subject.as_slice() {
            [types, type_name, subtype, child, ..]
                if types == "types" && !child.starts_with("$annotation:") =>
            {
                Some((claim.file.as_str(), type_name.as_str(), subtype.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut subtypes = KeySubtypes::new();
    for claim in &ledger.claims {
        let [types, type_name, subtype, annotation] = claim.subject.as_slice() else {
            continue;
        };
        let Some(KeyFilter::Equals(key)) = key_filter(claim) else {
            continue;
        };
        let selects_by_key =
            types == "types" && annotation == "$annotation:type_key_filter" && is_subtype(subtype);
        if selects_by_key
            && !with_body.contains(&(claim.file.as_str(), type_name.as_str(), subtype.as_str()))
        {
            subtypes
                .entry((claim.file.as_str(), type_name.as_str(), key))
                .or_default()
                .insert(subtype.as_str());
        }
    }
    subtypes
}

/// The IDs of the claims under a subtype that a root key selects (`## type_key_filter`). Such a
/// subtype keeps the SDK-800 rule, so the always-read rule (SDK-716) does not reach its claims.
fn root_key_subtype_claims<'a>(ledger: &'a Ledger, file_types: &FileTypes) -> BTreeSet<&'a str> {
    let root_key_subtypes: BTreeSet<_> = ledger
        .claims
        .iter()
        .filter_map(|claim| match claim.subject.as_slice() {
            [types, type_name, subtype, annotation]
                if types == "types" && annotation == "$annotation:type_key_filter" =>
            {
                let name = subtype.strip_prefix("subtype[")?.strip_suffix(']')?;

                Some((claim.file.as_str(), type_name.as_str(), name))
            }
            _ => None,
        })
        .collect();
    ledger
        .claims
        .iter()
        .filter(|claim| {
            let Some(type_name) = claim_type(claim, file_types) else {
                return false;
            };

            claim
                .conditions
                .iter()
                .flat_map(|condition| subtype_terms(condition))
                .any(|name| root_key_subtypes.contains(&(claim.file.as_str(), type_name, name)))
        })
        .map(|claim| claim.id.as_str())
        .collect()
}

/// The subtype names that a `subtype[…]` condition tests, without negation: `subtype[a&!b]`
/// tests `a` and `b`.
pub(crate) fn subtype_terms(condition: &str) -> impl Iterator<Item = &str> {
    condition
        .strip_prefix("subtype[")
        .and_then(|terms| terms.strip_suffix(']'))
        .into_iter()
        .flat_map(|terms| terms.split('&'))
        .map(|term| term.trim_start_matches('!'))
}

/// The `subtype[…]` conditions, in order.
pub(crate) fn subtypes(conditions: &[String]) -> impl Iterator<Item = &str> {
    conditions
        .iter()
        .filter(|condition| is_subtype(condition))
        .map(String::as_str)
}
