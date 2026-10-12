//! Project Atlas rule records onto config questions without comparing answers.

use super::{Answer, Evidence, Gap, Origin, Projection, Status};
use crate::{
    ledger::{Claim, Ledger},
    snapshot,
};
use pdx_native::Basis;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type ClaimIndex<'a> = BTreeMap<(&'a str, &'a str, Vec<&'a str>, Vec<&'a str>), BTreeSet<&'a str>>;
/// Config subtypes whose whole selector is a root key, by (file, type, root key).
type KeySubtypes<'a> = BTreeMap<(&'a str, &'a str, &'a str), BTreeSet<&'a str>>;
/// The config types that load from each content directory, as (type, file).
type RegistryTypes = BTreeMap<String, BTreeSet<(String, String)>>;

/// The lines of a config type that state how its loader reads files and names definitions.
const LOADER_LINES: &[&str] = &[
    "path",
    "path_extension",
    "path_strict",
    "path_file",
    "name_field",
    "skip_root_key",
    "type_per_file",
    "name_from_file",
];
/// The loader lines that the form naming a type's definitions answers.
const NAMING_LINES: &[&str] = &[
    "name_field",
    "skip_root_key",
    "type_per_file",
    "name_from_file",
];
/// The config file that writes each directory's duplicate-definition priority.
const OVERRIDES: &str = "overrides.cwt";

fn claim_index(ledger: &Ledger) -> ClaimIndex<'_> {
    let mut index = ClaimIndex::new();
    for claim in &ledger.claims {
        index
            .entry((
                &claim.file,
                &claim.property,
                claim.subject.iter().map(String::as_str).collect(),
                claim.conditions.iter().map(String::as_str).collect(),
            ))
            .or_default()
            .insert(&claim.question);
    }
    index
}

pub(super) fn project(ledger: &Ledger, rules: &snapshot::Snapshot) -> Result<Projection, String> {
    project_with_gap_facets(ledger, rules, true)
}

pub(super) fn project_for_comparison(
    ledger: &Ledger,
    rules: &snapshot::Snapshot,
) -> Result<Projection, String> {
    project_with_gap_facets(ledger, rules, false)
}

fn project_with_gap_facets(
    ledger: &Ledger,
    rules: &snapshot::Snapshot,
    project_related_gap_facets: bool,
) -> Result<Projection, String> {
    snapshot::verify(rules)?;
    let [build] = rules.applicability.builds.as_slice() else {
        return Err("Rule snapshot requires one exact Native build".into());
    };
    let target = serde_json::to_value(build)
        .map_err(|error| error.to_string())?
        .as_str()
        .ok_or("Native build id is not a string")?
        .to_owned();
    let registry_types = registry_types(ledger);
    let command_files = command_files(ledger);
    let claims = claim_index(ledger);
    let key_subtypes = key_subtypes(ledger);
    let language = super::language_subject::index(ledger, rules);
    let subjects: BTreeMap<_, _> = rules
        .subjects
        .iter()
        .map(|subject| (subject.id.as_str(), subject))
        .collect();
    let selectors = TypeSelectors::new(
        ledger,
        &registry_types,
        &identified_fields(&rules.rules, &subjects),
        &named_root_keys(&rules.rules, &subjects),
    );

    let mut projection = Projection {
        snapshot_id: format!("{}@{}", rules.snapshot.name, rules.snapshot.version),
        target: target.clone(),
        answers: Vec::new(),
        gaps: Vec::new(),
    };
    for rule in &rules.rules {
        let conditions = config_conditions(
            &key_subtypes,
            &selectors,
            subjects[rule.subject.as_str()],
            &rule.conditions,
        );
        let subject = subjects[rule.subject.as_str()];
        let mut answers = match (&conditions, rule.property.as_str()) {
            (Some(_), "definition_names") => {
                naming_answers(&claims, &selectors, subject, &rule.answer)
            }
            (Some(conditions), _) => questions(
                &claims,
                &selectors,
                &command_files,
                subject,
                &rule.property,
                conditions,
                false,
            )
            .into_iter()
            .map(|question| (question, rule.answer.clone()))
            .collect(),
            (None, _) => Vec::new(),
        };
        let conditions = conditions.unwrap_or_else(|| rule.conditions.clone());
        if rule.conditions.is_empty() {
            answers.extend(
                selectors
                    .language_questions(&language, &rule.id)
                    .into_iter()
                    .map(|question| (question, rule.answer.clone())),
            );
        }
        if answers.is_empty() {
            answers.push((format!("atlas:{}", rule.id), rule.answer.clone()));
        }
        let mut evidence = Vec::new();
        for link in &rule.evidence {
            let key = &rules
                .answers
                .get(&link.answer)
                .ok_or("Missing Native answer")?
                .source;
            let source = rules.sources.get(key).ok_or("Missing Native source")?;
            if source.build != *build {
                return Err(format!("Source build differs from snapshot target: {key}"));
            }
            evidence.push(Evidence {
                id: format!("{}@{key}", rule.id),
                method: source.method.clone(),
                target: target.clone(),
                qualified: source.basis != Basis::Recorded,
                origin: Origin::Engine,
            });
        }
        for (question, value) in answers {
            if let Some(gap) = selectors.selector_gap(&question, &conditions) {
                projection.gaps.push(gap);
                continue;
            }
            projection.answers.push(Answer {
                question,
                conditions: conditions.clone(),
                value,
                status: if rule.property == "value_form"
                    && rule.answer.get("alternatives").is_some()
                {
                    Status::Partial
                } else {
                    Status::Supported
                },
                evidence: evidence.clone(),
            });
        }
    }
    for gap in &rules.gaps {
        let mut questions = questions(
            &claims,
            &selectors,
            &command_files,
            subjects[gap.subject.as_str()],
            &gap.property,
            &[],
            project_related_gap_facets,
        );
        questions.extend(selectors.language_questions(&language, &gap.id));
        let questions = if questions.is_empty() {
            vec![format!("atlas:{}", gap.id)]
        } else {
            questions
        };
        for question in questions {
            if let Some(selector_gap) = selectors.selector_gap(&question, &[]) {
                projection.gaps.push(selector_gap);
            }
            projection.gaps.push(Gap {
                question,
                conditions: Vec::new(),
                reason: gap.reason.clone(),
            });
        }
    }
    projection.gaps.sort_by(|left, right| {
        (&left.question, &left.conditions, &left.reason).cmp(&(
            &right.question,
            &right.conditions,
            &right.reason,
        ))
    });
    projection.gaps.dedup_by(|left, right| {
        left.question == right.question
            && left.conditions == right.conditions
            && left.reason == right.reason
    });
    Ok(projection)
}

/// How a config type selects its definitions in its content directory.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Selector {
    /// The directory's root definitions: a type alone in its directory, or the one type of a shared
    /// directory with no `type_key_filter = KEY` and no `skip_root_key`. A `<>` filter only
    /// excludes keys, so it does not select another type.
    Root,
    /// Definitions nested in the root type's definitions, at this field path of the registry:
    /// `skip_root_key = any`, a `base_type` that names the root type, and `type_key_filter = KEY`
    /// for the last key. Native answers a nested definition only as a field under its owner.
    Nested(String),
    /// Root statements with this key: `type_key_filter = KEY` and no `skip_root_key`, where
    /// Native's loader names a `RootKey(KEY)` form. Only the type's loader lines join it; its other
    /// questions keep the gap `reason`.
    RootKey { key: String, reason: String },
    /// A selector that no Native answer identifies; the text is the gap reason.
    Unresolved(String),
}

/// The selector of each config type, and of each question that belongs to a type in a directory
/// with several types.
struct TypeSelectors<'a> {
    /// Each config type with its file and selector, by content directory.
    directories: BTreeMap<&'a str, Vec<(&'a str, &'a str, Selector)>>,
    /// The selector of each question's type, for types in a directory with several types.
    questions: BTreeMap<&'a str, Selector>,
    /// The loader-line questions (`path`, `path_extension`, …) of each type.
    loader_lines: BTreeSet<&'a str>,
    /// Each type that `path_file` selects, by the content path of its one file.
    single_files: BTreeMap<String, Vec<(&'a str, &'a str)>>,
}

impl<'a> TypeSelectors<'a> {
    /// `identified_fields` holds each (registry, field path) whose existence Native establishes
    /// without conditions; a nested type is identified only by its field. `named_root_keys`
    /// holds each (registry, root key) that Native's loader names as a form of its own.
    fn new(
        ledger: &'a Ledger,
        registry_types: &'a RegistryTypes,
        identified_fields: &BTreeSet<(&str, &str)>,
        named_root_keys: &BTreeSet<(&str, &str)>,
    ) -> Self {
        let selector_lines = selector_lines(ledger);
        let no_lines = SelectorLines::default();
        let mut directories = BTreeMap::new();
        let mut single_files: BTreeMap<_, Vec<_>> = BTreeMap::new();

        for (directory, types) in registry_types {
            let types: Vec<_> = types
                .iter()
                .map(|(type_name, file)| {
                    let lines = selector_lines
                        .get(&(file.as_str(), type_name.as_str()))
                        .unwrap_or(&no_lines);

                    (type_name.as_str(), file.as_str(), lines)
                })
                .collect();

            let mut roots = types.iter().filter(|(_, _, lines)| lines.selects_root());
            let root = match (roots.next(), roots.next()) {
                (Some((type_name, _, _)), None) => Some(*type_name),
                _ => None,
            };

            let selected: Vec<_> = types
                .iter()
                .map(|&(type_name, file, lines)| {
                    let selector = if types.len() == 1 || root == Some(type_name) {
                        Selector::Root
                    } else if let Some(path) = lines
                        .nested_path(root)
                        .filter(|path| identified_fields.contains(&(directory, path)))
                    {
                        Selector::Nested(path)
                    } else if let Some(key) = lines
                        .root_key()
                        .filter(|key| named_root_keys.contains(&(directory, key)))
                    {
                        Selector::RootKey {
                            key: key.into(),
                            reason: lines.unresolved_reason(),
                        }
                    } else {
                        Selector::Unresolved(lines.unresolved_reason())
                    };

                    if let Some(file_name) = lines.path_file {
                        single_files
                            .entry(format!("{directory}/{file_name}"))
                            .or_default()
                            .push((type_name, file));
                    }

                    (type_name, file, selector)
                })
                .collect();

            directories.insert(directory.as_str(), selected);
        }

        let shared_types: BTreeMap<_, _> = directories
            .values()
            .filter(|types| types.len() > 1)
            .flatten()
            .map(|(type_name, file, selector)| ((*file, *type_name), selector))
            .collect();
        let file_types = super::file_types(ledger);
        let questions = ledger
            .claims
            .iter()
            .filter_map(|claim| {
                let type_name = super::claim_type(claim, &file_types[&claim.file])?;
                let selector = shared_types.get(&(claim.file.as_str(), type_name))?;

                Some((claim.question.as_str(), (*selector).clone()))
            })
            .collect();
        let loader_lines = ledger
            .claims
            .iter()
            .filter(|claim| {
                matches!(
                    claim.subject.as_slice(),
                    [types, _, key] if types == "types" && LOADER_LINES.contains(&key.as_str())
                ) && claim.property == "loader_path"
            })
            .map(|claim| claim.question.as_str())
            .collect();

        Self {
            directories,
            questions,
            loader_lines,
            single_files,
        }
    }

    /// The gap that blocks a question whose type has an unresolved selector.
    fn selector_gap(&self, question: &str, conditions: &[String]) -> Option<Gap> {
        let reason = match self.questions.get(question)? {
            Selector::Unresolved(reason) => reason,
            Selector::RootKey { reason, .. } if !self.loader_lines.contains(question) => reason,
            _ => return None,
        };

        Some(Gap {
            question: question.into(),
            conditions: conditions.to_vec(),
            reason: reason.clone(),
        })
    }

    /// The `path_file` questions of the types whose one file is `registry`.
    fn single_file_questions(&self, claims: &ClaimIndex<'_>, registry: &str) -> Vec<String> {
        self.single_files
            .get(registry)
            .into_iter()
            .flatten()
            .flat_map(|(type_name, file)| {
                let path = vec!["types", *type_name, "path_file"];

                claim_questions(claims, file, "loader_path", path, &[])
            })
            .collect()
    }

    /// The language questions of a rule or gap. A nested type's naming lines name its own
    /// definitions, not the registry items that Native's derived names describe.
    fn language_questions(
        &self,
        language: &BTreeMap<String, BTreeSet<String>>,
        id: &str,
    ) -> Vec<String> {
        language
            .get(id)
            .into_iter()
            .flatten()
            .filter(|question| {
                !matches!(
                    self.questions.get(question.as_str()),
                    Some(Selector::Nested(_))
                )
            })
            .cloned()
            .collect()
    }
}

/// Each (registry, field path) whose existence a rule establishes without conditions.
fn identified_fields<'a>(
    rules: &'a [snapshot::Rule],
    subjects: &BTreeMap<&str, &'a snapshot::Subject>,
) -> BTreeSet<(&'a str, &'a str)> {
    rules
        .iter()
        .filter(|rule| rule.property == "existence" && rule.conditions.is_empty())
        .filter_map(|rule| {
            let subject = subjects[rule.subject.as_str()];

            Some((subject.registry.as_deref()?, subject.field.as_deref()?))
        })
        .collect()
}

/// Each (registry, root key) that a registry's `definition_names` rule names as a form of its own.
fn named_root_keys<'a>(
    rules: &'a [snapshot::Rule],
    subjects: &BTreeMap<&str, &'a snapshot::Subject>,
) -> BTreeSet<(&'a str, &'a str)> {
    rules
        .iter()
        .filter(|rule| rule.property == "definition_names")
        .filter_map(|rule| Some((subjects[rule.subject.as_str()], rule.answer.as_array()?)))
        .filter(|(subject, _)| subject.kind == snapshot::SubjectKind::Registry)
        .flat_map(|(subject, forms)| {
            forms.iter().filter_map(|form| {
                let key = form.get("statement")?.get("RootKey")?.as_str()?;

                Some((subject.registry.as_deref()?, key))
            })
        })
        .collect()
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
struct SelectorLines<'a> {
    key_filter: Option<KeyFilter<'a>>,
    /// `skip_root_key` keys in order: one key at 0, or the items of a `{ … }` list.
    skip_root_key: BTreeMap<usize, &'a str>,
    base_type: Option<&'a str>,
    /// The one file of the type's directory that the type loads from.
    path_file: Option<&'a str>,
}

impl SelectorLines<'_> {
    /// Whether these lines leave the type among the directory's root definitions.
    fn selects_root(&self) -> bool {
        self.skip_root_key.is_empty()
            && matches!(self.key_filter, None | Some(KeyFilter::Excludes(_)))
    }

    /// The root key that alone selects the type's definitions.
    fn root_key(&self) -> Option<&str> {
        let Some(KeyFilter::Equals(key)) = self.key_filter else {
            return None;
        };

        self.skip_root_key.is_empty().then_some(key)
    }

    /// The field path of a nested definition below the `root` type's definitions.
    fn nested_path(&self, root: Option<&str>) -> Option<String> {
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
    fn unresolved_reason(&self) -> String {
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

/// The content directory of a config `path`. Two config types write their path without `game/`;
/// the directory is the same.
pub(super) fn config_directory(path: &str) -> &str {
    let path = path.trim_matches('"');
    path.strip_prefix("game/").unwrap_or(path)
}

pub(super) fn registry_types(ledger: &Ledger) -> RegistryTypes {
    let mut candidates = RegistryTypes::new();
    let file_types = super::file_types(ledger);
    for claim in &ledger.claims {
        if claim.property != "loader_path"
            || claim.subject.len() != 3
            || claim.subject[0] != "types"
            || claim.subject[2] != "path"
        {
            continue;
        }
        let type_name = &claim.subject[1];
        let registry = config_directory(&claim.config_answer);
        if file_types[&claim.file].contains(type_name) {
            candidates
                .entry(registry.into())
                .or_default()
                .insert((type_name.clone(), claim.file.clone()));
        }
    }
    candidates
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
        let selects_by_key = types == "types"
            && annotation == "$annotation:type_key_filter"
            && subtype.starts_with("subtype[");
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

/// A rule's conditions in the config's terms. A `root_key[KEY]` condition becomes the one subtype
/// of the registry's root type that the key alone selects; `None` when no such subtype is unique,
/// or when another subtype or root-key condition goes with it.
fn config_conditions(
    key_subtypes: &KeySubtypes<'_>,
    type_selectors: &TypeSelectors<'_>,
    subject: &snapshot::Subject,
    conditions: &[String],
) -> Option<Vec<String>> {
    let selectors = conditions
        .iter()
        .filter(|condition| condition.starts_with("root_key[") || condition.starts_with("subtype["))
        .count();
    conditions
        .iter()
        .map(|condition| {
            let Some(key) = condition
                .strip_prefix("root_key[")
                .and_then(|rest| rest.strip_suffix(']'))
            else {
                return Some(condition.clone());
            };
            if selectors > 1 {
                return None;
            }
            let types = type_selectors
                .directories
                .get(subject.registry.as_deref()?)?;
            let mut subtypes = types
                .iter()
                .filter(|(_, _, selector)| *selector == Selector::Root)
                .flat_map(|(type_name, file, _)| {
                    key_subtypes
                        .get(&(*file, *type_name, key))
                        .into_iter()
                        .flatten()
                });
            match (subtypes.next(), subtypes.next()) {
                (Some(subtype), None) => Some((*subtype).to_owned()),
                _ => None,
            }
        })
        .collect()
}

fn questions(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    command_files: &BTreeMap<String, BTreeSet<String>>,
    subject: &snapshot::Subject,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    match (subject.kind, &subject.name, &subject.field) {
        (snapshot::SubjectKind::Registry | snapshot::SubjectKind::Field, _, field) => {
            let Some(registry) = subject.registry.as_deref() else {
                return Vec::new();
            };
            let mut questions = match (subject.kind, property) {
                (snapshot::SubjectKind::Registry, "duplicate_rule") => {
                    return priority_questions(claims, registry);
                }
                (snapshot::SubjectKind::Registry, "loader_path") => {
                    selectors.single_file_questions(claims, registry)
                }
                _ => Vec::new(),
            };
            let Some(types) = selectors.directories.get(registry) else {
                return questions;
            };

            questions.extend(types.iter().flat_map(|(type_name, file, selector)| {
                let field = field.as_deref();
                let (field, property) = match selector {
                    Selector::Nested(path) => match (field, property) {
                        (None, "loader_path" | "extension" | "recursion") => (None, property),
                        (Some(field), "existence") if field == path => (None, property),
                        _ => return Vec::new(),
                    },
                    Selector::Root | Selector::RootKey { .. } | Selector::Unresolved(_) => {
                        (field, property)
                    }
                };

                type_questions(
                    claims, type_name, file, field, property, conditions, gap_facet,
                )
            }));

            questions
        }
        (snapshot::SubjectKind::Argument, Some(command), Some(field)) => {
            let root = format!("alias[{command}]");
            let Some(files) = command_files.get(&root) else {
                return Vec::new();
            };

            files
                .iter()
                .flat_map(|file| {
                    field_questions(claims, file, &root, field, property, conditions, gap_facet)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The `overrides.cwt` questions about a directory's priority: that it has one, and its value.
fn priority_questions(claims: &ClaimIndex<'_>, registry: &str) -> Vec<String> {
    ["field_existence", "value_form"]
        .into_iter()
        .flat_map(|property| {
            claim_questions(
                claims,
                OVERRIDES,
                property,
                vec!["priorities", registry],
                &[],
            )
        })
        .collect()
}

/// The naming-line questions that a `definition_names` rule answers, each with the form that
/// names its type's definitions as the answer. A registry rule lists the root forms and answers
/// the Root and RootKey types; a field rule holds one nested form and answers the nested type at
/// that path.
fn naming_answers(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    subject: &snapshot::Subject,
    answer: &Value,
) -> Vec<(String, Value)> {
    let Some(types) = subject
        .registry
        .as_deref()
        .and_then(|registry| selectors.directories.get(registry))
    else {
        return Vec::new();
    };

    types
        .iter()
        .flat_map(|(type_name, file, selector)| {
            let Some((lines, form)) = type_form(selector, subject.field.as_deref(), answer) else {
                return Vec::new();
            };

            lines
                .iter()
                .flat_map(|line| {
                    let path = vec!["types", *type_name, line];

                    claim_questions(claims, file, "loader_path", path, &[])
                })
                .map(|question| (question, form.clone()))
                .collect()
        })
        .collect()
}

/// The form that names the definitions of a type with `selector`, and the naming lines that it
/// answers. A known empty list of root forms answers only `name_field`: the loader names no
/// definition, which says nothing of how files or wrapper keys hold them.
fn type_form(
    selector: &Selector,
    field: Option<&str>,
    answer: &Value,
) -> Option<(&'static [&'static str], Value)> {
    let statement = |form: &&Value| form.get("statement").cloned();

    match (selector, field) {
        (Selector::Nested(path), Some(field)) if path == field => {
            Some((NAMING_LINES, answer.clone()))
        }
        (Selector::Root, None) => {
            let forms = answer.as_array()?;

            if forms.is_empty() {
                return Some((&["name_field"], answer.clone()));
            }

            let root = forms
                .iter()
                .find(|form| statement(form) == Some(json!("Root")));
            let form = match (root, forms.as_slice()) {
                (Some(root), _) => root,
                (None, [only]) => only,
                (None, _) => return None,
            };

            Some((NAMING_LINES, form.clone()))
        }
        (Selector::RootKey { key, .. }, None) => {
            let form = answer
                .as_array()?
                .iter()
                .find(|form| statement(form) == Some(json!({ "RootKey": key })))?;

            Some((NAMING_LINES, form.clone()))
        }
        _ => None,
    }
}

/// The config files that declare each command, by its `alias[{kind}:{name}]` root.
fn command_files(ledger: &Ledger) -> BTreeMap<String, BTreeSet<String>> {
    let mut files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for claim in &ledger.claims {
        if let [root] = claim.subject.as_slice()
            && claim.property == "command_existence"
        {
            files
                .entry(root.clone())
                .or_default()
                .insert(claim.file.clone());
        }
    }

    files
}

fn type_questions(
    claims: &ClaimIndex<'_>,
    type_name: &str,
    file: &str,
    field: Option<&str>,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    let (path, ledger_property): (Vec<&str>, &str) = match (field, property) {
        (None, "existence") => (vec!["types", type_name], "type_existence"),
        (None, "loader_path") => (vec!["types", type_name, "path"], "loader_path"),
        (None, "extension") => (vec!["types", type_name, "path_extension"], "loader_path"),
        (None, "recursion") => (vec!["types", type_name, "path_strict"], "loader_path"),
        (None, "definition_names") => {
            return NAMING_LINES
                .iter()
                .flat_map(|line| {
                    let path = vec!["types", type_name, line];

                    claim_questions(claims, file, "loader_path", path, conditions)
                })
                .collect();
        }
        (Some(field), _) => {
            let root = type_name.trim_start_matches("type[").trim_end_matches(']');

            return field_questions(claims, file, root, field, property, conditions, gap_facet);
        }
        _ => return Vec::new(),
    };

    claim_questions(claims, file, ledger_property, path, conditions)
}

/// Config questions about the field at `field` below the config node `root`: a type's
/// definition or a command's block.
fn field_questions(
    claims: &ClaimIndex<'_>,
    file: &str,
    root: &str,
    field: &str,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    let (annotation, ledger_property) = match property {
        "existence" => (None, "field_existence"),
        "value_form" | "value_form.unresolved" => (None, "value_form"),
        "reference" | "nested_grammar" if gap_facet => (None, "value_form"),
        "occurrences.minimum" => (Some("$annotation:cardinality"), "cardinality_minimum"),
        "occurrences.maximum" | "repeat_behavior" => {
            (Some("$annotation:cardinality"), "cardinality_maximum")
        }
        "scope_context" => (Some("$annotation:replace_scopes"), "scope_context"),
        "read_scope" => (Some("$annotation:push_scope"), "scope_context"),
        _ => return Vec::new(),
    };
    let mut path = vec![root];
    path.extend(
        conditions
            .iter()
            .filter(|condition| condition.starts_with("subtype["))
            .map(String::as_str),
    );
    path.extend(field.split('/'));
    path.extend(annotation);

    claim_questions(claims, file, ledger_property, path, conditions)
}

fn claim_questions(
    claims: &ClaimIndex<'_>,
    file: &str,
    ledger_property: &str,
    path: Vec<&str>,
    conditions: &[String],
) -> Vec<String> {
    claims
        .get(&(
            file,
            ledger_property,
            path,
            conditions.iter().map(String::as_str).collect(),
        ))
        .map(|questions| {
            questions
                .iter()
                .map(|question| (*question).to_owned())
                .collect()
        })
        .unwrap_or_default()
}
