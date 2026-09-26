//! Project Atlas rule records onto config questions without comparing answers.

use super::{Answer, Evidence, Gap, Origin, Projection, Status};
use crate::{ledger::Ledger, snapshot};
use pdx_native::Basis;
use std::collections::{BTreeMap, BTreeSet};

type ClaimIndex<'a> = BTreeMap<(&'a str, &'a str, Vec<&'a str>, Vec<&'a str>), BTreeSet<&'a str>>;

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
    let claims = claim_index(ledger);
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
        let mut questions = questions(
            &claims,
            &registry_types,
            subjects[rule.subject.as_str()],
            &rule.property,
            &rule.conditions,
            false,
        );
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
            let subject = subjects[rule.subject.as_str()];
            let ambiguous = subject
                .registry
                .as_ref()
                .and_then(|registry| registry_types.get(registry))
                .is_some_and(|types| types.len() > 1);
            if ambiguous && !question.starts_with("atlas:") {
                projection.gaps.push(Gap { question, conditions: rule.conditions.clone(), reason: "The registry directory maps to several config types; the applicable type is unresolved".into() });
                continue;
            }
            projection.answers.push(Answer {
                question,
                conditions: rule.conditions.clone(),
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
            projection.gaps.push(Gap {
                question,
                conditions: Vec::new(),
                reason: gap.reason.clone(),
            });
        }
    }
    Ok(projection)
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

fn questions(
    claims: &ClaimIndex<'_>,
    registry_types: &BTreeMap<String, BTreeSet<(String, String)>>,
    subject: &snapshot::Subject,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    if !matches!(
        subject.kind,
        snapshot::SubjectKind::Registry | snapshot::SubjectKind::Field
    ) {
        return Vec::new();
    }
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
                subject.field.as_deref(),
                property,
                conditions,
                gap_facet,
            )
        })
        .collect()
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
        (Some(field), "existence") => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
            ],
            "field_existence",
        ),
        (Some(field), "value_form" | "value_form.unresolved") => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
            ],
            "value_form",
        ),
        (Some(field), "reference" | "nested_grammar") if gap_facet => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
            ],
            "value_form",
        ),
        (Some(field), "occurrences.minimum") => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
                "$annotation:cardinality",
            ],
            "cardinality_minimum",
        ),
        (Some(field), "occurrences.maximum") => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
                "$annotation:cardinality",
            ],
            "cardinality_maximum",
        ),
        (Some(field), "scope_context") => (
            vec![
                type_name.trim_start_matches("type[").trim_end_matches(']'),
                field,
                "$annotation:replace_scopes",
            ],
            "scope_context",
        ),
        _ => return Vec::new(),
    };
    let path = if field.is_some() {
        let mut expanded = Vec::with_capacity(path.len() + conditions.len());
        expanded.push(path[0]);
        expanded.extend(
            conditions
                .iter()
                .filter(|condition| condition.starts_with("subtype["))
                .map(String::as_str),
        );
        for segment in &path[1..] {
            expanded.extend(segment.split('/'));
        }
        expanded
    } else {
        path
    };
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
