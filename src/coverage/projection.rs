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

/// The lines of a config type that state how its loader selects files.
const FILE_SELECTION_LINES: &[&str] = &["path", "path_extension", "path_strict", "path_file"];
/// The lines of a config type that the form naming its definitions answers.
const NAMING_LINES: &[&str] = &[
    "name_field",
    "skip_root_key",
    "type_per_file",
    "name_from_file",
];
/// The registries whose root fields are root statements: Native answers each event kind
/// (`country_event`) as a root field of `events` (Native's `registry-fields.md`, "Event kinds").
/// In every other registry a root field is a key inside a definition, except the keys of
/// [`ROOT_STATEMENT_KEYS`].
const ROOT_STATEMENT_REGISTRIES: &[&str] = &["events"];
/// Root fields that Native answers as root statements of a type of their own, not as keys inside
/// a definition, by (registry, key): `random_list` of `common/planet_classes` (Native's
/// `registry-fields.md`, "Root-key owners (SDK-813)"). The type is the one whose selector is
/// `type_key_filter = KEY`.
const ROOT_STATEMENT_KEYS: &[(&str, &str)] = &[("common/planet_classes", "random_list")];
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
    let alias_blocks = AliasBlocks::new(ledger);
    let enum_domains = EnumDomains::new(ledger);
    let counted_minimums = counted_minimums(ledger);
    let language = super::language_subject::index(ledger, rules);
    let subjects: BTreeMap<_, _> = rules
        .subjects
        .iter()
        .map(|subject| (subject.id.as_str(), subject))
        .collect();
    let subtype_claims = SubtypeClaims::new(ledger, rules, &subjects);
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
        let statement = root_statement(&key_subtypes, &selectors, subjects[rule.subject.as_str()]);
        let (subject, rule_conditions) = match &statement {
            Some((subject, key)) => (
                subject,
                [rule.conditions.clone(), vec![key.clone()]].concat(),
            ),
            None => (subjects[rule.subject.as_str()], rule.conditions.clone()),
        };
        let conditions = config_conditions(&key_subtypes, &selectors, subject, &rule_conditions);
        let mut unconditional = conditions.as_deref().map_or_else(Vec::new, |conditions| {
            alias_blocks.questions(
                &claims,
                &selectors,
                subject,
                &rule.property,
                conditions,
                false,
            )
        });

        if statement.is_some() && rule.conditions.is_empty() {
            unconditional.extend(definition_questions(
                &claims,
                &selectors,
                subject,
                &rule.property,
                false,
            ));
        }

        let mut unconditional: Vec<_> = unconditional
            .into_iter()
            .map(|question| (question, rule.answer.clone()))
            .collect();

        if let (Some(conditions), "domain") = (&conditions, rule.property.as_str()) {
            let value_forms =
                field_value_forms(&claims, &selectors, &command_files, subject, conditions);
            let (selected, unselected): (Vec<_>, Vec<_>) = value_forms
                .into_iter()
                .partition(|question| selectors.selector_gap(question, conditions).is_none());

            unconditional.extend(enum_domains.answers(&selected, &rule.answer));
            projection
                .gaps
                .extend(enum_domains.selector_gaps(&selectors, &unselected, conditions));
        }

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
        let subtype_answers = match statement {
            None if rule.conditions.is_empty() => {
                subtype_claims.questions(&selectors, &command_files, subject, &rule.property, false)
            }
            _ => Vec::new(),
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
        if answers.is_empty() && unconditional.is_empty() && subtype_answers.is_empty() {
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
        let subtype_evidence = subtype_claims.evidence(&rule.subject, &evidence);
        let answers = answers
            .into_iter()
            .map(|(question, value)| (question, conditions.clone(), value, &evidence))
            .chain(
                unconditional
                    .into_iter()
                    .map(|(question, value)| (question, Vec::new(), value, &evidence)),
            )
            .chain(subtype_answers.into_iter().map(|(question, conditions)| {
                (question, conditions, rule.answer.clone(), &subtype_evidence)
            }));
        for (question, conditions, value, evidence) in answers {
            if let Some(gap) = selectors.selector_gap(&question, &conditions) {
                projection.gaps.push(gap);
                continue;
            }
            if rule.property == "occurrences.minimum"
                && value == json!(1)
                && counted_minimums.contains(question.as_str())
            {
                projection.gaps.push(Gap {
                    question,
                    conditions,
                    reason: "Native establishes that a definition must contain this field, not a \
                             count above 1"
                        .into(),
                });
                continue;
            }
            projection.answers.push(Answer {
                question,
                conditions,
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
        let statement = root_statement(&key_subtypes, &selectors, subjects[gap.subject.as_str()]);
        let (subject, conditions) = match &statement {
            Some((subject, key)) => (
                subject,
                config_conditions(
                    &key_subtypes,
                    &selectors,
                    subject,
                    std::slice::from_ref(key),
                )
                .unwrap_or_default(),
            ),
            None => (subjects[gap.subject.as_str()], Vec::new()),
        };
        let mut questions: Vec<_> = questions(
            &claims,
            &selectors,
            &command_files,
            subject,
            &gap.property,
            &conditions,
            project_related_gap_facets,
        )
        .into_iter()
        .map(|question| (question, conditions.clone()))
        .collect();
        if statement.is_none() {
            questions.extend(subtype_claims.questions(
                &selectors,
                &command_files,
                subject,
                &gap.property,
                project_related_gap_facets,
            ));
        }

        let definition = match statement {
            Some(_) => definition_questions(
                &claims,
                &selectors,
                subject,
                &gap.property,
                project_related_gap_facets,
            ),
            None => Vec::new(),
        };
        let enum_members = match gap.property.as_str() {
            "domain" => {
                let value_forms =
                    field_value_forms(&claims, &selectors, &command_files, subject, &conditions);

                enum_domains.member_questions(&value_forms)
            }
            _ => Vec::new(),
        };
        let unconditional = selectors
            .language_questions(&language, &gap.id)
            .into_iter()
            .chain(alias_blocks.questions(
                &claims,
                &selectors,
                subject,
                &gap.property,
                &conditions,
                project_related_gap_facets,
            ))
            .chain(definition)
            .chain(enum_members);
        questions.extend(unconditional.map(|question| (question, Vec::new())));

        if questions.is_empty() {
            questions.push((format!("atlas:{}", gap.id), Vec::new()));
        }

        for (question, conditions) in questions {
            if let Some(selector_gap) = selectors.selector_gap(&question, &conditions) {
                projection.gaps.push(selector_gap);
            }
            projection.gaps.push(Gap {
                question,
                conditions,
                reason: gap.reason.clone(),
            });
        }
    }
    let definition_gaps = definition_line_gaps(&claims, &selectors, &subjects, &projection.answers);
    projection.gaps.extend(definition_gaps);
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
    /// Root statements with this key, as `RootKey` selects them, where Native answers the key as
    /// a root field of the registry ([`ROOT_STATEMENT_KEYS`]). The field answers the type and
    /// its fields answer the type's fields; no other field of the registry reaches the type.
    Statement(String),
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
    /// The file-selection and naming-line questions of each type.
    loader_lines: BTreeSet<&'a str>,
    /// The `path_file` questions of each type, which only a registry named by the one file answers.
    single_file_lines: BTreeSet<&'a str>,
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
                    let statement = lines.root_key().filter(|key| {
                        ROOT_STATEMENT_KEYS.contains(&(directory.as_str(), *key))
                            && identified_fields.contains(&(directory, *key))
                    });
                    let selector = if let Some(key) = statement {
                        Selector::Statement(key.into())
                    } else if types.len() == 1 || root == Some(type_name) {
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
                    [types, _, line] if types == "types"
                        && (FILE_SELECTION_LINES.contains(&line.as_str())
                            || NAMING_LINES.contains(&line.as_str()))
                ) && claim.property == "loader_path"
            })
            .map(|claim| claim.question.as_str())
            .collect();

        let single_file_lines = ledger
            .claims
            .iter()
            .filter(|claim| {
                matches!(claim.subject.as_slice(), [types, _, line] if types == "types" && line == "path_file")
                    && claim.property == "loader_path"
            })
            .map(|claim| claim.question.as_str())
            .collect();

        Self {
            directories,
            questions,
            loader_lines,
            single_file_lines,
            single_files,
        }
    }

    /// The gap that blocks a question whose type has an unresolved selector. A `path_file`
    /// question is exempt: only the registry named by its one file answers it, whatever other
    /// types share the folder.
    fn selector_gap(&self, question: &str, conditions: &[String]) -> Option<Gap> {
        if self.single_file_lines.contains(question) {
            return None;
        }

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

pub(crate) fn registry_types(ledger: &Ledger) -> RegistryTypes {
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

/// A field of a registry in [`ROOT_STATEMENT_REGISTRIES`] whose first key alone selects a subtype
/// of the registry's root type names a root statement with that key. The result is the subject
/// below the statement and the `root_key[KEY]` condition that selects it.
fn root_statement(
    key_subtypes: &KeySubtypes<'_>,
    selectors: &TypeSelectors<'_>,
    subject: &snapshot::Subject,
) -> Option<(snapshot::Subject, String)> {
    let registry = subject.registry.as_deref()?;

    if subject.kind != snapshot::SubjectKind::Field
        || !ROOT_STATEMENT_REGISTRIES.contains(&registry)
    {
        return None;
    }

    let field = subject.field.as_deref()?;
    let (key, below) = match field.split_once('/') {
        Some((key, below)) => (key, Some(below)),
        None => (field, None),
    };
    let condition = format!("root_key[{key}]");

    config_conditions(
        key_subtypes,
        selectors,
        subject,
        std::slice::from_ref(&condition),
    )?;

    let below = snapshot::Subject {
        field: below.map(Into::into),
        ..subject.clone()
    };

    Some((below, condition))
}

/// The claims of the root type's unconditional definition block (`event = { … }`) that a root
/// statement's own `property` answers. `subject` is the part below the statement: with no field,
/// it is the statement itself. Every statement is a definition, so each one answers these claims
/// and a gap on any of them blocks the credit.
fn definition_questions(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    subject: &snapshot::Subject,
    property: &str,
    gap_facet: bool,
) -> Vec<String> {
    let (None, Some(registry)) = (&subject.field, subject.registry.as_deref()) else {
        return Vec::new();
    };
    let Some((annotation, ledger_property)) = field_property(property, gap_facet) else {
        return Vec::new();
    };
    let Some(types) = selectors.directories.get(registry) else {
        return Vec::new();
    };

    types
        .iter()
        .filter(|(_, _, selector)| *selector == Selector::Root)
        .flat_map(|(type_name, file, _)| {
            let root = type_name.trim_start_matches("type[").trim_end_matches(']');
            let mut path = vec![root];
            path.extend(annotation);

            claim_questions(claims, file, ledger_property, path, &[])
        })
        .collect()
}

/// Whether the `definition_names` of `registry` answers the definition line of a type with
/// `selector`. The definitions of a statement's own type (SDK-818) and of the root type of the
/// event kinds (SDK-815) are answered by the statements themselves. Native identifies a nested
/// type only as a field under its owner (SDK-778), so its naming form answers no definition line.
fn names_definition_line(registry: &str, selector: &Selector) -> bool {
    match selector {
        Selector::Root => !ROOT_STATEMENT_REGISTRIES.contains(&registry),
        Selector::RootKey { .. } => true,
        Selector::Nested(_) | Selector::Statement(_) | Selector::Unresolved(_) => false,
    }
}

/// The claims of a type's definition line (`ship_size = { … }`), outside any subtype.
fn definition_line_questions(
    claims: &ClaimIndex<'_>,
    type_name: &str,
    file: &str,
    ledger_property: &str,
) -> Vec<String> {
    let root = type_name.trim_start_matches("type[").trim_end_matches(']');

    claim_questions(claims, file, ledger_property, vec![root], &[])
}

/// The reason why a definition line of a type whose directory Native lists has no answer, for
/// each of its claims that no unconditional rule answers, with the type's selector gap: Native states neither the value form of a root
/// definition nor one scope for a whole definition. The definition lines that a root statement
/// answers (SDK-815, SDK-818) keep their answers.
fn definition_line_gaps(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    subjects: &BTreeMap<&str, &snapshot::Subject>,
    answers: &[Answer],
) -> Vec<Gap> {
    const SCOPE: &str = "Native states the scope of each block that a definition reads, on that \
                         block's field; no answer states one scope for the whole definition";
    const LINES: &[(&str, Option<&str>, &str)] = &[
        (
            "value_form",
            None,
            "No Native answer states the value form of a root definition; a field of a definition \
             does not show that every definition is a block",
        ),
        ("scope_context", Some("$annotation:push_scope"), SCOPE),
        ("scope_context", Some("$annotation:replace_scopes"), SCOPE),
    ];
    let answered: BTreeSet<_> = answers
        .iter()
        .filter(|answer| answer.conditions.is_empty())
        .map(|answer| answer.question.as_str())
        .collect();
    let registries: BTreeSet<_> = subjects
        .values()
        .filter(|subject| subject.kind == snapshot::SubjectKind::Registry)
        .filter_map(|subject| subject.registry.as_deref())
        .collect();

    registries
        .into_iter()
        .filter_map(|registry| selectors.directories.get(registry))
        .flatten()
        .flat_map(|(type_name, file, _)| {
            let root = type_name.trim_start_matches("type[").trim_end_matches(']');

            LINES
                .iter()
                .flat_map(move |(property, annotation, reason)| {
                    let mut path = vec![root];
                    path.extend(annotation);

                    claim_questions(claims, file, property, path, &[])
                        .into_iter()
                        .map(|question| Gap {
                            question,
                            conditions: Vec::new(),
                            reason: (*reason).into(),
                        })
                })
        })
        .filter(|gap| !answered.contains(gap.question.as_str()))
        .flat_map(|gap| {
            let selector_gap = selectors.selector_gap(&gap.question, &[]);

            selector_gap.into_iter().chain([gap])
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

            let statement_key = field
                .as_deref()
                .map(|field| field.split('/').next().unwrap_or(field))
                .filter(|key| ROOT_STATEMENT_KEYS.contains(&(registry, *key)));

            questions.extend(types.iter().flat_map(|(type_name, file, selector)| {
                let field = field.as_deref();
                let (field, property) = match selector {
                    Selector::Nested(path) => match (field, property) {
                        (None, "loader_path" | "extension" | "recursion") => (None, property),
                        (Some(field), "existence") if field == path => (None, property),
                        _ => return Vec::new(),
                    },
                    Selector::Statement(key) => match (field, property) {
                        (None, "existence") => return Vec::new(),
                        (None, _) => (None, property),
                        (Some(field), _) if field == key => {
                            return statement_questions(
                                claims, type_name, file, property, gap_facet,
                            );
                        }
                        (Some(field), _) => match field.strip_prefix(&format!("{key}/")) {
                            Some(below) => (Some(below), property),
                            None => return Vec::new(),
                        },
                    },
                    // A root statement's fields are not keys of the directory's other types.
                    _ if statement_key.is_some() => return Vec::new(),
                    Selector::Root | Selector::RootKey { .. } | Selector::Unresolved(_) => {
                        (field, property)
                    }
                };

                let mut questions = type_questions(
                    claims, type_name, file, field, property, conditions, gap_facet,
                );

                if field.is_none()
                    && property == "definition_names"
                    && names_definition_line(registry, selector)
                {
                    questions.extend(definition_line_questions(
                        claims,
                        type_name,
                        file,
                        "field_existence",
                    ));
                }

                questions
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

/// The questions that a root statement's own `property` answers about the type it selects: its
/// existence is the type's, and its existence and value form are those of the type's definition
/// block.
fn statement_questions(
    claims: &ClaimIndex<'_>,
    type_name: &str,
    file: &str,
    property: &str,
    gap_facet: bool,
) -> Vec<String> {
    let mut questions = match property {
        "existence" => claim_questions(
            claims,
            file,
            "type_existence",
            vec!["types", type_name],
            &[],
        ),
        _ => Vec::new(),
    };
    let Some((annotation, ledger_property)) = field_property(property, gap_facet) else {
        return questions;
    };
    let mut path = vec![type_name.trim_start_matches("type[").trim_end_matches(']')];
    path.extend(annotation);
    questions.extend(claim_questions(claims, file, ledger_property, path, &[]));

    questions
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

/// The questions that a `definition_names` rule answers: each naming line, with the form that
/// names its type's definitions as the answer, and the definition line's existence, with `true`.
/// A registry rule lists the root forms and answers the Root and RootKey types; a field rule
/// holds one nested form and answers only the naming lines of the nested type at that path.
fn naming_answers(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    subject: &snapshot::Subject,
    answer: &Value,
) -> Vec<(String, Value)> {
    let Some(registry) = subject.registry.as_deref() else {
        return Vec::new();
    };
    let Some(types) = selectors.directories.get(registry) else {
        return Vec::new();
    };

    types
        .iter()
        .flat_map(|(type_name, file, selector)| {
            let Some((lines, form)) = type_form(selector, subject.field.as_deref(), answer) else {
                return Vec::new();
            };
            let naming = lines.iter().flat_map(|line| {
                let path = vec!["types", *type_name, line];

                claim_questions(claims, file, "loader_path", path, &[])
            });
            // A root form names the type's definitions, so they exist; a known empty list names
            // none, which says nothing of whether the type has any.
            let definitions = match form {
                Value::Object(_)
                    if subject.field.is_none() && names_definition_line(registry, selector) =>
                {
                    definition_line_questions(claims, type_name, file, "field_existence")
                }
                _ => Vec::new(),
            };

            naming
                .map(|question| (question, form.clone()))
                .chain(
                    definitions
                        .into_iter()
                        .map(|question| (question, json!(true))),
                )
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
                .find(|form| form.get("statement") == Some(&json!("Root")));
            let form = match (root, forms.as_slice()) {
                (Some(root), _) => root,
                (None, [only]) => only,
                (None, _) => return None,
            };

            Some((NAMING_LINES, form.clone()))
        }
        (Selector::RootKey { key, .. } | Selector::Statement(key), None) => {
            let form = answer
                .as_array()?
                .iter()
                .find(|form| form.get("statement") == Some(&json!({ "RootKey": key })))?;

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
        // A root statement's read scope: the `push_scope` of the subtype that its key selects.
        (None, "read_scope") => {
            let mut path = vec!["types", type_name];
            path.extend(subtypes(conditions));
            path.push("$annotation:push_scope");

            (path, "scope_context")
        }
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
    let Some((annotation, ledger_property)) = field_property(property, gap_facet) else {
        return Vec::new();
    };
    let mut path = node_path(root, conditions, field);
    path.extend(annotation);

    claim_questions(claims, file, ledger_property, path, conditions)
}

/// The annotation below a field's config node and the ledger property that a field property
/// answers, or `None` when no field claim answers it.
fn field_property(property: &str, gap_facet: bool) -> Option<(Option<&'static str>, &'static str)> {
    Some(match property {
        "existence" => (None, "field_existence"),
        "value_form" | "value_form.unresolved" => (None, "value_form"),
        "reference" | "nested_grammar" if gap_facet => (None, "value_form"),
        "occurrences.minimum" => (Some("$annotation:cardinality"), "cardinality_minimum"),
        "occurrences.maximum" | "repeat_behavior" => {
            (Some("$annotation:cardinality"), "cardinality_maximum")
        }
        "scope_context" => (Some("$annotation:replace_scopes"), "scope_context"),
        "read_scope" => (Some("$annotation:push_scope"), "scope_context"),
        _ => return None,
    })
}

/// The config path of the field at `field` below `root`, inside the subtypes of `conditions`.
fn node_path<'a>(root: &'a str, conditions: &'a [String], field: &'a str) -> Vec<&'a str> {
    let mut path = vec![root];
    path.extend(subtypes(conditions));
    path.extend(field.split('/'));

    path
}

/// The `subtype[…]` conditions, in order.
fn subtypes(conditions: &[String]) -> impl Iterator<Item = &str> {
    conditions
        .iter()
        .filter(|condition| condition.starts_with("subtype["))
        .map(String::as_str)
}

/// The claims under `subtype[…]` conditions that an always-read field answers (SDK-716). When
/// every read alternative of a field has the condition `Always`, Native reads it the same way on
/// every path, so its unconditioned existence, value form and maximum also hold under any subtype.
/// A subtype can require a field or change its scope, so no other property reaches these claims.
/// A subtype that a root key selects keeps the SDK-800 rule and takes nothing from this one.
struct SubtypeClaims<'a> {
    /// Each claim with a subtype condition, keyed by its path without the subtype segments and no
    /// conditions, so [`questions`] finds it as it finds the unconditioned claim at that path.
    index: ClaimIndex<'a>,
    /// The conditions of each question in `index`.
    conditions: BTreeMap<&'a str, &'a [String]>,
    /// The subjects whose read alternatives all have the condition `Always`, each with whether
    /// qualified evidence establishes that premise.
    always_read: BTreeMap<&'a str, bool>,
}

impl<'a> SubtypeClaims<'a> {
    /// The rule and gap properties whose unconditioned answer holds under any subtype. `reference`
    /// and `nested_grammar` reach the value form only as gap facets.
    const PROPERTIES: &'static [&'static str] = &[
        "existence",
        "value_form",
        "value_form.unresolved",
        "reference",
        "nested_grammar",
        "occurrences.maximum",
        "repeat_behavior",
    ];

    fn new(
        ledger: &'a Ledger,
        rules: &'a snapshot::Snapshot,
        subjects: &BTreeMap<&str, &snapshot::Subject>,
    ) -> Self {
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
        let file_types = super::file_types(ledger);
        let mut index = ClaimIndex::new();
        let mut conditions = BTreeMap::new();

        for claim in &ledger.claims {
            if claim.conditions.is_empty() {
                continue;
            }

            let type_name = super::claim_type(claim, &file_types[&claim.file]);
            let selected_by_root_key = type_name.is_some_and(|type_name| {
                claim
                    .conditions
                    .iter()
                    .flat_map(|condition| subtype_terms(condition))
                    .any(|name| root_key_subtypes.contains(&(claim.file.as_str(), type_name, name)))
            });

            if selected_by_root_key {
                continue;
            }

            let base_path = claim
                .subject
                .iter()
                .map(String::as_str)
                .filter(|segment| !segment.starts_with("subtype["))
                .collect();

            index
                .entry((&claim.file, &claim.property, base_path, Vec::new()))
                .or_default()
                .insert(&claim.question);
            conditions.insert(claim.question.as_str(), claim.conditions.as_slice());
        }

        Self {
            index,
            conditions,
            always_read: always_read_fields(rules, subjects),
        }
    }

    /// The subtype questions, with their conditions, that the unconditioned `property` of
    /// `subject` answers; none unless `subject` is always read and `property` holds under any
    /// subtype.
    fn questions(
        &self,
        selectors: &TypeSelectors<'_>,
        command_files: &BTreeMap<String, BTreeSet<String>>,
        subject: &snapshot::Subject,
        property: &str,
        gap_facet: bool,
    ) -> Vec<(String, Vec<String>)> {
        if !self.always_read.contains_key(subject.id.as_str())
            || !Self::PROPERTIES.contains(&property)
        {
            return Vec::new();
        }

        questions(
            &self.index,
            selectors,
            command_files,
            subject,
            property,
            &[],
            gap_facet,
        )
        .into_iter()
        .map(|question| {
            let conditions = self.conditions[question.as_str()].to_vec();

            (question, conditions)
        })
        .collect()
    }

    /// The evidence of a subtype answer of `subject`: the rule's own `evidence`, qualified only
    /// when the always-read premise is qualified too, so a recorded premise earns no credit.
    fn evidence(&self, subject: &str, evidence: &[Evidence]) -> Vec<Evidence> {
        let premise_qualified = self.always_read.get(subject).copied().unwrap_or(false);

        evidence
            .iter()
            .map(|evidence| Evidence {
                qualified: evidence.qualified && premise_qualified,
                ..evidence.clone()
            })
            .collect()
    }
}

/// The subtype names that a `subtype[…]` condition tests, without negation: `subtype[a&!b]`
/// tests `a` and `b`.
fn subtype_terms(condition: &str) -> impl Iterator<Item = &str> {
    condition
        .strip_prefix("subtype[")
        .and_then(|terms| terms.strip_suffix(']'))
        .into_iter()
        .flat_map(|terms| terms.split('&'))
        .map(|term| term.trim_start_matches('!'))
}

/// The subjects whose `conditions` rule lists only `Always` alternatives, each with whether every
/// source of that rule is qualified. A subject that Native marks conditional, such as the child
/// of a conditional parent, is not read on every path, whatever its own alternatives.
fn always_read_fields<'a>(
    rules: &'a snapshot::Snapshot,
    subjects: &BTreeMap<&str, &snapshot::Subject>,
) -> BTreeMap<&'a str, bool> {
    rules
        .rules
        .iter()
        .filter(|rule| rule.property == "conditions" && rule.conditions.is_empty())
        .filter(|rule| subjects[rule.subject.as_str()].conditional != Some(true))
        .filter(|rule| {
            rule.answer.as_array().is_some_and(|alternatives| {
                !alternatives.is_empty() && alternatives.iter().all(|a| a == "Always")
            })
        })
        .map(|rule| {
            let qualified = !rule.evidence.is_empty()
                && rule.evidence.iter().all(|link| {
                    rules
                        .answers
                        .get(&link.answer)
                        .and_then(|answer| rules.sources.get(&answer.source))
                        .is_some_and(|source| source.basis != Basis::Recorded)
                });

            (rule.subject.as_str(), qualified)
        })
        .collect()
}

/// The blocks whose config entry is `alias_name[X] = alias_match_left[X]`. Each Native field in
/// such a block answers the claims at `alias[X:KEY]`, which hold wherever the alias is used, so
/// these answers have no conditions.
struct AliasBlocks<'a> {
    /// The alias that names the keys of each block, by (file, block path, conditions).
    aliases: BTreeMap<(&'a str, Vec<&'a str>, Vec<&'a str>), &'a str>,
    /// The config files that declare each `alias[X:KEY]` root.
    files: BTreeMap<&'a str, BTreeSet<&'a str>>,
}

impl<'a> AliasBlocks<'a> {
    fn new(ledger: &'a Ledger) -> Self {
        let mut aliases = BTreeMap::new();
        let mut files: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();

        for claim in &ledger.claims {
            let Some((last, block)) = claim.subject.split_last() else {
                continue;
            };

            if let Some(alias) = last
                .strip_prefix("alias_name[")
                .and_then(|alias| alias.strip_suffix(']'))
                && claim.property == "alias_factoring"
                && claim.config_answer == format!("alias_match_left[{alias}]")
            {
                aliases.insert(
                    (
                        claim.file.as_str(),
                        block.iter().map(String::as_str).collect(),
                        claim.conditions.iter().map(String::as_str).collect(),
                    ),
                    alias,
                );
            }

            if claim.subject[0].starts_with("alias[") {
                files
                    .entry(claim.subject[0].as_str())
                    .or_default()
                    .insert(claim.file.as_str());
            }
        }

        Self { aliases, files }
    }

    /// The `alias[X:KEY]` questions that a field of a registry's root type answers when its parent
    /// block takes its keys from alias `X`.
    fn questions(
        &self,
        claims: &ClaimIndex<'_>,
        selectors: &TypeSelectors<'_>,
        subject: &snapshot::Subject,
        property: &str,
        conditions: &[String],
        gap_facet: bool,
    ) -> Vec<String> {
        let (Some(registry), Some((block, key))) = (
            subject.registry.as_deref(),
            subject
                .field
                .as_deref()
                .and_then(|field| field.rsplit_once('/')),
        ) else {
            return Vec::new();
        };
        let Some((annotation, ledger_property)) = field_property(property, gap_facet) else {
            return Vec::new();
        };
        let condition_names: Vec<_> = conditions.iter().map(String::as_str).collect();
        let root_types = selectors
            .directories
            .get(registry)
            .into_iter()
            .flatten()
            .filter(|(_, _, selector)| *selector == Selector::Root);
        let mut questions = Vec::new();

        for (type_name, file, _) in root_types {
            let root = type_name.trim_start_matches("type[").trim_end_matches(']');
            let block_path = node_path(root, conditions, block);
            let Some(alias) = self
                .aliases
                .get(&(*file, block_path, condition_names.clone()))
            else {
                continue;
            };
            let alias_root = format!("alias[{alias}:{key}]");

            for alias_file in self.files.get(alias_root.as_str()).into_iter().flatten() {
                let mut path = vec![alias_root.as_str()];
                path.extend(annotation);

                questions.extend(claim_questions(
                    claims,
                    alias_file,
                    ledger_property,
                    path,
                    &[],
                ));
            }
        }

        questions
    }
}

/// The `value_form` questions of the config field that a field subject joins.
fn field_value_forms(
    claims: &ClaimIndex<'_>,
    selectors: &TypeSelectors<'_>,
    command_files: &BTreeMap<String, BTreeSet<String>>,
    subject: &snapshot::Subject,
    conditions: &[String],
) -> Vec<String> {
    questions(
        claims,
        selectors,
        command_files,
        subject,
        "value_form",
        conditions,
        false,
    )
}

/// The enums that config fields take their values from, and each enum's members. A field's
/// `domain` (the names it accepts) answers each member of the enum that the field's config value
/// names: whether the field accepts that name. The enum holds wherever it is used, so these
/// answers have no conditions, and a domain gap on any field that names the enum blocks them.
struct EnumDomains<'a> {
    /// The enum that each `value_form` question names, when its whole config value is `enum[X]`.
    enums: BTreeMap<&'a str, &'a str>,
    /// Each enum's member questions with the member's name, from every file that lists it.
    members: BTreeMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl<'a> EnumDomains<'a> {
    fn new(ledger: &'a Ledger) -> Self {
        let mut enums = BTreeMap::new();
        let mut members: BTreeMap<_, Vec<_>> = BTreeMap::new();

        for claim in &ledger.claims {
            match claim.property.as_str() {
                "value_form" => {
                    if let Some(name) = enum_name(&claim.config_answer) {
                        enums.insert(claim.question.as_str(), name);
                    }
                }
                "enum_member" => {
                    if let [.., name, _] = claim.subject.as_slice()
                        && let Some(name) = enum_name(name)
                    {
                        members
                            .entry(name)
                            .or_default()
                            .push((claim.question.as_str(), claim.config_answer.as_str()));
                    }
                }
                _ => {}
            }
        }

        Self { enums, members }
    }

    /// The members of the enums that `value_forms` name.
    fn members(&self, value_forms: &[String]) -> impl Iterator<Item = (&'a str, &'a str)> {
        let names: BTreeSet<_> = value_forms
            .iter()
            .filter_map(|question| self.enums.get(question.as_str()).copied())
            .collect();

        names
            .into_iter()
            .flat_map(|name| self.members.get(name).into_iter().flatten().copied())
    }

    /// The selector gap of each `value_forms` question, on the members of the enum that it names:
    /// a field of a type that no Native answer identifies does not answer the enum.
    fn selector_gaps(
        &self,
        selectors: &TypeSelectors<'_>,
        value_forms: &[String],
        conditions: &[String],
    ) -> Vec<Gap> {
        value_forms
            .iter()
            .filter_map(|question| selectors.selector_gap(question, conditions))
            .flat_map(|gap| {
                self.member_questions(std::slice::from_ref(&gap.question))
                    .into_iter()
                    .map(move |question| Gap {
                        question,
                        conditions: Vec::new(),
                        reason: gap.reason.clone(),
                    })
            })
            .collect()
    }

    fn member_questions(&self, value_forms: &[String]) -> Vec<String> {
        self.members(value_forms)
            .map(|(question, _)| question.to_owned())
            .collect()
    }

    /// Whether `domain`, a list of accepted names, holds each member.
    fn answers(&self, value_forms: &[String], domain: &Value) -> Vec<(String, Value)> {
        let accepted: BTreeSet<_> = domain
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();

        self.members(value_forms)
            .map(|(question, name)| (question.to_owned(), json!(accepted.contains(name))))
            .collect()
    }
}

/// The `cardinality_minimum` questions whose config minimum is above 1: a count, which a field's
/// requirement does not answer.
fn counted_minimums(ledger: &Ledger) -> BTreeSet<&str> {
    ledger
        .claims
        .iter()
        .filter(|claim| claim.property == "cardinality_minimum")
        .filter(|claim| {
            claim
                .config_answer
                .parse::<u64>()
                .is_ok_and(|minimum| minimum > 1)
        })
        .map(|claim| claim.question.as_str())
        .collect()
}

/// `X` of a config `enum[X]`.
fn enum_name(text: &str) -> Option<&str> {
    text.strip_prefix("enum[")?.strip_suffix(']')
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
