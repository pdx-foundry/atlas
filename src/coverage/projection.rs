//! Project Atlas rule records onto config questions without comparing answers.

use super::{Answer, Evidence, Gap, Origin, Projection, Status};
use crate::{ledger::Ledger, snapshot};
use pdx_native::Basis;
use std::collections::{BTreeMap, BTreeSet};

type ClaimIndex<'a> = BTreeMap<(&'a str, &'a str, Vec<&'a str>, Vec<&'a str>), BTreeSet<&'a str>>;
/// Config subtypes whose whole selector is a root key, by (file, type, root key).
type KeySubtypes<'a> = BTreeMap<(&'a str, &'a str, &'a str), BTreeSet<&'a str>>;

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
    let mut projection = Projection {
        snapshot_id: format!("{}@{}", rules.snapshot.name, rules.snapshot.version),
        target: target.clone(),
        answers: Vec::new(),
        gaps: Vec::new(),
    };
    let subjects: BTreeMap<_, _> = rules
        .subjects
        .iter()
        .map(|subject| (subject.id.as_str(), subject))
        .collect();
    for rule in &rules.rules {
        let conditions = config_conditions(
            &key_subtypes,
            &registry_types,
            subjects[rule.subject.as_str()],
            &rule.conditions,
        );
        let mut questions = match &conditions {
            Some(conditions) => questions(
                &claims,
                &registry_types,
                &command_files,
                subjects[rule.subject.as_str()],
                &rule.property,
                conditions,
                false,
            ),
            None => Vec::new(),
        };
        let conditions = conditions.unwrap_or_else(|| rule.conditions.clone());
        if rule.conditions.is_empty() {
            questions.extend(language.get(&rule.id).into_iter().flatten().cloned());
        }
        let questions = if questions.is_empty() {
            vec![format!("atlas:{}", rule.id)]
        } else {
            questions
        };
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
        for question in questions {
            if let Some(gap) = mapping_gap(
                subjects[rule.subject.as_str()],
                &registry_types,
                &question,
                &conditions,
            ) {
                projection.gaps.push(gap);
                continue;
            }
            projection.answers.push(Answer {
                question,
                conditions: conditions.clone(),
                value: rule.answer.clone(),
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
            &registry_types,
            &command_files,
            subjects[gap.subject.as_str()],
            &gap.property,
            &[],
            project_related_gap_facets,
        );
        questions.extend(language.get(&gap.id).into_iter().flatten().cloned());
        let questions = if questions.is_empty() {
            vec![format!("atlas:{}", gap.id)]
        } else {
            questions
        };
        for question in questions {
            if let Some(mapping) = mapping_gap(
                subjects[gap.subject.as_str()],
                &registry_types,
                &question,
                &[],
            ) {
                projection.gaps.push(mapping);
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

fn mapping_gap(
    subject: &snapshot::Subject,
    registry_types: &BTreeMap<String, BTreeSet<(String, String)>>,
    question: &str,
    conditions: &[String],
) -> Option<Gap> {
    let ambiguous = subject
        .registry
        .as_ref()
        .and_then(|registry| registry_types.get(registry))
        .is_some_and(|types| types.len() > 1);
    if !ambiguous || question.starts_with("atlas:") {
        return None;
    }
    Some(Gap {
        question: question.into(),
        conditions: conditions.to_vec(),
        reason:
            "The registry directory maps to several config types; the applicable type is unresolved"
                .into(),
    })
}

fn registry_types(ledger: &Ledger) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let mut candidates: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let existing_types = ledger
        .claims
        .iter()
        .filter(|claim| {
            claim.property == "type_existence"
                && claim.subject.len() == 2
                && claim.subject[0] == "types"
        })
        .map(|claim| (claim.file.as_str(), claim.subject[1].as_str()))
        .collect::<BTreeSet<_>>();
    for claim in &ledger.claims {
        if claim.property != "loader_path"
            || claim.subject.len() != 3
            || claim.subject[0] != "types"
            || claim.subject[2] != "path"
        {
            continue;
        }
        let type_name = &claim.subject[1];
        let Some(registry) = claim.config_answer.trim_matches('"').strip_prefix("game/") else {
            continue;
        };
        let has_type = existing_types.contains(&(claim.file.as_str(), type_name.as_str()));
        if has_type {
            candidates
                .entry(registry.into())
                .or_default()
                .insert((type_name.clone(), claim.file.clone()));
        }
    }
    candidates
}

/// Config subtypes that `## type_key_filter = KEY` selects and whose definition body is empty, so
/// the root key is their whole selector. A subtype that also tests a field is left out.
fn key_subtypes(ledger: &Ledger) -> KeySubtypes<'_> {
    let field_tested = ledger
        .claims
        .iter()
        .filter(|claim| claim.property == "conditional_constraint")
        .filter_map(|claim| match claim.subject.as_slice() {
            [types, type_name, subtype, _, ..] if types == "types" => {
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
        // `config_answer` drops a `<>` filter, so read the key from the source text.
        let Some(key) = claim
            .source_text
            .strip_prefix("## type_key_filter =")
            .map(str::trim)
        else {
            continue;
        };
        let selects_by_key = types == "types"
            && annotation == "$annotation:type_key_filter"
            && subtype.starts_with("subtype[")
            && !key.is_empty()
            && !key.contains(char::is_whitespace)
            && !key.contains('{');
        if selects_by_key
            && !field_tested.contains(&(claim.file.as_str(), type_name.as_str(), subtype.as_str()))
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
/// that the key alone selects; `None` when no such subtype is unique for the rule's registry.
fn config_conditions(
    key_subtypes: &KeySubtypes<'_>,
    registry_types: &BTreeMap<String, BTreeSet<(String, String)>>,
    subject: &snapshot::Subject,
    conditions: &[String],
) -> Option<Vec<String>> {
    conditions
        .iter()
        .map(|condition| {
            let Some(key) = condition
                .strip_prefix("root_key[")
                .and_then(|rest| rest.strip_suffix(']'))
            else {
                return Some(condition.clone());
            };
            let types = registry_types.get(subject.registry.as_ref()?)?;
            let mut subtypes = types.iter().flat_map(|(type_name, file)| {
                key_subtypes
                    .get(&(file.as_str(), type_name.as_str(), key))
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
    registry_types: &BTreeMap<String, BTreeSet<(String, String)>>,
    command_files: &BTreeMap<String, BTreeSet<String>>,
    subject: &snapshot::Subject,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    match (subject.kind, &subject.name, &subject.field) {
        (snapshot::SubjectKind::Registry | snapshot::SubjectKind::Field, _, field) => {
            let Some(types) = subject
                .registry
                .as_ref()
                .and_then(|registry| registry_types.get(registry))
            else {
                return Vec::new();
            };

            types
                .iter()
                .flat_map(|(type_name, file)| {
                    type_questions(
                        claims,
                        type_name,
                        file,
                        field.as_deref(),
                        property,
                        conditions,
                        gap_facet,
                    )
                })
                .collect()
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
