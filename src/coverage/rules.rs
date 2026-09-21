//! Project Atlas rule records onto config questions without comparing answers.

use super::{Answer, Evidence, Gap, Origin, Snapshot, Status};
use crate::{ledger::Ledger, snapshot};
use pdx_native::Basis;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn project(ledger: &Ledger, rules: &snapshot::Snapshot) -> Result<Snapshot, String> {
    project_with_gap_facets(ledger, rules, true)
}

pub(super) fn project_for_comparison(
    ledger: &Ledger,
    rules: &snapshot::Snapshot,
) -> Result<Snapshot, String> {
    project_with_gap_facets(ledger, rules, false)
}

fn project_with_gap_facets(
    ledger: &Ledger,
    rules: &snapshot::Snapshot,
    project_related_gap_facets: bool,
) -> Result<Snapshot, String> {
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
    let mut projection = Snapshot {
        kind: "atlas_coverage".into(),
        format_version: 1,
        snapshot_id: format!("{}@{}", rules.snapshot.name, rules.snapshot.version),
        target: target.clone(),
        answers: Vec::new(),
        gaps: Vec::new(),
    };
    for rule in &rules.rules {
        let questions = questions(
            ledger,
            &registry_types,
            &rule.subject,
            &rule.property,
            &rule.conditions,
            false,
        );
        let questions = if questions.is_empty() {
            vec![format!("atlas:{}", rule.id)]
        } else {
            questions
        };
        let mut evidence = Vec::new();
        for link in &rule.evidence {
            let source = rules
                .sources
                .get(&link.source)
                .ok_or("Missing Native source")?;
            if source.build != *build {
                return Err(format!(
                    "Source build differs from snapshot target: {}",
                    link.source
                ));
            }
            evidence.push(Evidence {
                id: format!("{}@{}", rule.id, link.source),
                method: source.method.clone(),
                target: target.clone(),
                qualified: source.basis != Basis::Recorded,
                origin: Origin::Engine,
            });
        }
        for question in questions {
            projection.answers.push(Answer {
                question,
                conditions: rule.conditions.clone(),
                value: rule.answer.clone(),
                status: Status::Supported,
                evidence: evidence.clone(),
            });
        }
    }
    for gap in &rules.gaps {
        let questions = questions(
            ledger,
            &registry_types,
            &gap.subject,
            &gap.property,
            &[],
            project_related_gap_facets,
        );
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

fn registry_types(ledger: &Ledger) -> BTreeMap<String, (String, String)> {
    let mut types = BTreeMap::new();
    for claim in &ledger.claims {
        if claim.property != "loader_path"
            || claim.subject.len() != 3
            || claim.subject[0] != "types"
            || claim.subject[2] != "path"
        {
            continue;
        }
        let type_name = &claim.subject[1];
        if type_name == "type[swapped_tradition]" {
            continue;
        }
        let Some(registry) = claim.config_answer.trim_matches('"').strip_prefix("game/") else {
            continue;
        };
        let has_type = ledger.claims.iter().any(|candidate| {
            candidate.file == claim.file
                && candidate.property == "type_existence"
                && candidate.subject == ["types", type_name]
        });
        if has_type {
            types.insert(registry.into(), (type_name.clone(), claim.file.clone()));
        }
    }
    types
}

fn questions(
    ledger: &Ledger,
    registry_types: &BTreeMap<String, (String, String)>,
    subject: &str,
    property: &str,
    conditions: &[String],
    gap_facet: bool,
) -> Vec<String> {
    let (registry, field) = if let Some(registry) = subject.strip_prefix("registry:") {
        (registry, None)
    } else if let Some(path) = subject.strip_prefix("field:") {
        let Some((registry, field)) = path.rsplit_once('/') else {
            return Vec::new();
        };
        (registry, Some(field))
    } else {
        return Vec::new();
    };
    let Some((type_name, file)) = registry_types.get(registry) else {
        return Vec::new();
    };
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
        (Some(field), "value_form") => (
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
        expanded.extend_from_slice(&path[1..]);
        expanded
    } else {
        path
    };
    ledger
        .claims
        .iter()
        .filter(|claim| {
            claim.file == *file
                && claim.property == ledger_property
                && claim
                    .subject
                    .iter()
                    .map(String::as_str)
                    .eq(path.iter().copied())
                && claim.conditions == conditions
        })
        .map(|claim| claim.question.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
