//! Registry, field and fixture-outcome rules.

use super::{
    EvidenceLink, Rule, Snapshot, Subject, SubjectKind, evidence, field_id, gap, registry_id, rule,
};
use crate::extraction::Extraction;
use pdx_native::{
    Answer, Completeness, DiagnosticCoverage, DiagnosticJoin, Disposal, Field, FixtureFieldOutcome,
    FixtureObservation, FixtureRuntime, FixtureStorage, ReaderKind,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Property families that registry assembly can establish.
pub(super) const PROPERTIES: &[&str] = &[
    "existence",
    "loader_path",
    "occurrences.parser_accepted",
    "value_form",
];

pub(super) fn assemble(snapshot: &mut Snapshot, extraction: &Extraction) -> Result<(), String> {
    for registry in extraction.fields.keys() {
        assemble_registry(snapshot, extraction, registry)?;
        assemble_fields(snapshot, extraction, registry)?;
    }
    assemble_outcomes(snapshot, extraction)
}

fn assemble_registry(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<(), String> {
    let id = registry_id(registry);
    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: SubjectKind::Registry,
        registry: Some(registry.into()),
        field: None,
        conditional: None,
        name: None,
    });
    match &extraction.registries {
        Ok(answer) if answer.value.iter().any(|found| found.name == registry) => {
            let evidence = evidence(
                snapshot,
                "registries",
                answer,
                format!("answers.registries:{registry}"),
                Some(registry),
            )?;
            rule(snapshot, &id, "existence", json!(true), evidence.clone());
            rule(snapshot, &id, "loader_path", json!(registry), evidence);
        }
        Ok(answer) => {
            let evidence = evidence(
                snapshot,
                "registries",
                answer,
                format!("answers.registries:{registry}"),
                Some(registry),
            )?;
            gap(
                snapshot,
                &id,
                "existence",
                "Native did not name this registry in the bounded answer",
                None,
                answer.gaps.clone(),
                vec![evidence.clone()],
            );
            gap(
                snapshot,
                &id,
                "loader_path",
                "Native did not establish this registry's content directory",
                None,
                answer.gaps.clone(),
                vec![evidence],
            );
        }
        Err(error) => {
            for property in ["existence", "loader_path"] {
                gap(
                    snapshot,
                    &id,
                    property,
                    format!("Native registry question failed: {error:?}"),
                    None,
                    Vec::new(),
                    Vec::new(),
                );
            }
        }
    }
    Ok(())
}

fn assemble_fields(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<(), String> {
    let registry_subject = registry_id(registry);
    let Some(result) = extraction.fields.get(registry) else {
        gap(
            snapshot,
            &registry_subject,
            "fields",
            "Native field answer is missing",
            None,
            Vec::new(),
            Vec::new(),
        );
        return Ok(());
    };
    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            gap(
                snapshot,
                &registry_subject,
                "fields",
                format!("Native field question failed: {error:?}"),
                None,
                Vec::new(),
                Vec::new(),
            );
            return Ok(());
        }
    };
    if answer.completeness == Completeness::Partial {
        let evidence = evidence(
            snapshot,
            &format!("registry_fields/{registry}"),
            answer,
            format!("answers.fields.{registry}"),
            None,
        )?;
        gap(
            snapshot,
            &registry_subject,
            "fields_complete",
            "Native's root-field search is partial; undiscovered fields remain unknown",
            None,
            answer.gaps.clone(),
            vec![evidence],
        );
    }
    for field in &answer.value {
        assemble_field(snapshot, registry, field, answer)?;
    }
    Ok(())
}

fn assemble_field(
    snapshot: &mut Snapshot,
    registry: &str,
    field: &Field,
    answer: &Answer<Vec<Field>>,
) -> Result<(), String> {
    let id = field_id(registry, &field.name);
    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: SubjectKind::Field,
        registry: Some(registry.into()),
        field: Some(field.name.clone()),
        conditional: Some(field.conditional),
        name: None,
    });
    let evidence = evidence(
        snapshot,
        &format!("registry_fields/{registry}"),
        answer,
        format!("answers.fields.{registry}:{}", field.name),
        Some(&field.name),
    )?;
    rule(snapshot, &id, "existence", json!(true), evidence.clone());
    assemble_field_value_form(snapshot, &id, field, answer, &evidence)?;
    assemble_field_gaps(snapshot, &id, field, &evidence);
    Ok(())
}

fn assemble_field_value_form(
    snapshot: &mut Snapshot,
    id: &str,
    field: &Field,
    answer: &Answer<Vec<Field>>,
    evidence: &EvidenceLink,
) -> Result<(), String> {
    if field.conditional {
        gap(
            snapshot,
            id,
            "conditions",
            "The reader depends on state not established by this field key",
            None,
            answer.gaps.clone(),
            vec![evidence.clone()],
        );
        gap(
            snapshot,
            id,
            "value_form",
            "The reader's conditions are not established",
            None,
            answer.gaps.clone(),
            vec![evidence.clone()],
        );
    } else if let Some(reader_id) = &field.reader.id {
        let reader_id = serde_json::to_value(reader_id)
            .map_err(|error| error.to_string())?
            .as_str()
            .ok_or("Native reader id is not a string")?
            .to_owned();
        let shape = match field.reader.kind {
            ReaderKind::Boolean => Some(("boolean", "boolean")),
            ReaderKind::Integer => Some(("integer", "integer")),
            ReaderKind::FixedPoint => Some(("number", "number")),
            ReaderKind::String => Some(("string", "string")),
            ReaderKind::Reference => Some(("reference", "string")),
            ReaderKind::Block => {
                rule(
                    snapshot,
                    id,
                    "value_form",
                    json!({"form":"block","reader":reader_id}),
                    evidence.clone(),
                );
                gap(
                    snapshot,
                    id,
                    "nested_grammar",
                    "A block reader does not establish its nested grammar",
                    None,
                    Vec::new(),
                    vec![evidence.clone()],
                );
                gap(
                    snapshot,
                    id,
                    "scope_context",
                    "A block reader does not establish scope context",
                    None,
                    Vec::new(),
                    vec![evidence.clone()],
                );
                None
            }
            ReaderKind::Unknown => None,
            _ => None,
        };
        if let Some((form, schema_type)) = shape {
            let schema_name = format!("reader-{reader_id}");
            let schema = json!({"type":schema_type});
            if let Some(existing) = snapshot.schemas.definitions.get(&schema_name) {
                if existing != &schema {
                    return Err(format!("Conflicting shapes for reader {reader_id}"));
                }
            } else {
                snapshot
                    .schemas
                    .definitions
                    .insert(schema_name.clone(), schema);
            }
            rule(
                snapshot,
                id,
                "value_form",
                json!({"form":form,"schema":format!("#/schemas/$defs/{schema_name}"),"reader":reader_id}),
                evidence.clone(),
            );
        } else if field.reader.kind != ReaderKind::Block {
            gap(
                snapshot,
                id,
                "value_form",
                "Native did not establish this reader's value form",
                None,
                answer
                    .gaps
                    .iter()
                    .filter(|native_gap| native_gap.subject.as_deref() == Some(field.name.as_str()))
                    .cloned()
                    .collect(),
                vec![evidence.clone()],
            );
        }
    } else {
        gap(
            snapshot,
            id,
            "value_form",
            "Native did not establish a reader identity",
            None,
            answer
                .gaps
                .iter()
                .filter(|native_gap| native_gap.subject.as_deref() == Some(field.name.as_str()))
                .cloned()
                .collect(),
            vec![evidence.clone()],
        );
    }
    Ok(())
}

fn assemble_field_gaps(snapshot: &mut Snapshot, id: &str, field: &Field, evidence: &EvidenceLink) {
    if matches!(
        field.reader.kind,
        ReaderKind::String | ReaderKind::Reference
    ) {
        gap(
            snapshot,
            id,
            "reference",
            "A string-like reader does not establish a lookup category",
            None,
            Vec::new(),
            vec![evidence.clone()],
        );
    }
    for property in ["occurrences.minimum", "occurrences.maximum"] {
        gap(
            snapshot,
            id,
            property,
            "Finite fixture observations do not establish an occurrence bound",
            None,
            Vec::new(),
            vec![evidence.clone()],
        );
    }
}

#[derive(Default)]
struct Occurrences {
    counts: BTreeSet<usize>,
    final_values: BTreeSet<String>,
    rejected: Vec<Value>,
    evidence: Vec<EvidenceLink>,
    gap_evidence: Vec<EvidenceLink>,
    reasons: BTreeMap<&'static str, Vec<String>>,
}

enum ParserOutcome {
    Accepted {
        count: usize,
        final_value: Option<&'static str>,
    },
    Rejected(Vec<Value>),
    Unavailable {
        property: &'static str,
        reason: String,
    },
}

fn classify_parser_outcome(
    observation: &FixtureObservation,
    outcome: &FixtureFieldOutcome,
) -> ParserOutcome {
    let diagnostics: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter_map(|index| observation.diagnostics.get(*index))
        .collect();
    if diagnostics.len() != outcome.diagnostics.len() {
        return ParserOutcome::Unavailable {
            property: "validation",
            reason: "Native's diagnostic index did not resolve".into(),
        };
    }
    if observation
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.join, DiagnosticJoin::Unavailable(_)))
    {
        return ParserOutcome::Unavailable {
            property: "validation",
            reason: "A fixture diagnostic was not joined to a source location".into(),
        };
    }
    if !diagnostics.is_empty() {
        return ParserOutcome::Rejected(diagnostics.into_iter().map(|diagnostic| {
            json!({"definition":outcome.question.definition,"text":diagnostic.text,"join":diagnostic.join})
        }).collect());
    }
    match &observation.diagnostic_coverage {
        DiagnosticCoverage::Complete { .. } => {}
        DiagnosticCoverage::Unavailable(reason) => {
            return ParserOutcome::Unavailable {
                property: "validation",
                reason: reason.clone(),
            };
        }
        DiagnosticCoverage::NotRequested => {
            return ParserOutcome::Unavailable {
                property: "validation",
                reason: "Parser diagnostics were not requested".into(),
            };
        }
    }
    match &outcome.storage {
        FixtureStorage::String {
            occurrences,
            final_value,
            completeness: Completeness::Complete,
        } => {
            let final_value = final_value.as_ref().and_then(|final_value| {
                let first = occurrences.first().map(|record| &record.value);
                let last = occurrences.last().map(|record| &record.value);
                if last == Some(final_value) {
                    Some("last")
                } else if first == Some(final_value) {
                    Some("first")
                } else {
                    None
                }
            });
            ParserOutcome::Accepted {
                count: occurrences.len(),
                final_value,
            }
        }
        FixtureStorage::String {
            completeness: Completeness::Partial,
            ..
        } => ParserOutcome::Unavailable {
            property: "storage",
            reason: "Parser storage was partial".into(),
        },
        FixtureStorage::Unavailable(reason) => ParserOutcome::Unavailable {
            property: "storage",
            reason: reason.clone(),
        },
    }
}

fn assemble_outcomes(snapshot: &mut Snapshot, extraction: &Extraction) -> Result<(), String> {
    let mut outcomes: BTreeMap<String, Occurrences> = BTreeMap::new();
    for session in &extraction.sessions {
        if !matches!(
            &session.disposal,
            Ok(Disposal::Confirmed | Disposal::NotApplicable)
        ) {
            gap(
                snapshot,
                &registry_id(session.registry),
                &format!("fixture.{}", session.name),
                format!(
                    "Native fixture session disposal failed: {:?}",
                    session.disposal
                ),
                None,
                Vec::new(),
                Vec::new(),
            );
            continue;
        }
        let observation = match &session.observation {
            Ok(answer) => answer,
            Err(error) => {
                gap(
                    snapshot,
                    &registry_id(session.registry),
                    &format!("fixture.{}", session.name),
                    format!("Native fixture question failed: {error:?}"),
                    None,
                    Vec::new(),
                    Vec::new(),
                );
                continue;
            }
        };
        for outcome in &observation.value.field_outcomes {
            let id = field_id(&outcome.question.registry, &outcome.question.field);
            if !snapshot.subjects.iter().any(|subject| subject.id == id) {
                continue;
            }
            let entry = outcomes.entry(id.clone()).or_default();
            let outcome_evidence = evidence(
                snapshot,
                &format!("observe_fixture/{}", session.name),
                observation,
                format!(
                    "answers.sessions.{}.fixture:{}:{}",
                    session.name, outcome.question.definition, outcome.question.field
                ),
                None,
            )?;
            entry.gap_evidence.push(outcome_evidence.clone());
            match classify_parser_outcome(&observation.value, outcome) {
                ParserOutcome::Accepted { count, final_value } => {
                    entry.counts.insert(count);
                    if let Some(final_value) = final_value {
                        entry.final_values.insert(final_value.into());
                    }
                    entry.evidence.push(outcome_evidence.clone());
                }
                ParserOutcome::Rejected(rejected) => {
                    entry.rejected.extend(rejected);
                    entry.evidence.push(outcome_evidence.clone());
                }
                ParserOutcome::Unavailable { property, reason } => {
                    entry.reasons.entry(property).or_default().push(reason)
                }
            }
            if let FixtureRuntime::Unavailable(reason) = &outcome.runtime {
                let runtime_evidence = evidence(
                    snapshot,
                    &format!("observe_fixture/{}", session.name),
                    observation,
                    format!("answers.sessions.{}.fixture.runtime", session.name),
                    None,
                )?;
                gap(
                    snapshot,
                    &id,
                    "runtime",
                    reason.clone(),
                    None,
                    observation.gaps.clone(),
                    vec![runtime_evidence],
                );
            }
        }
    }
    for (id, mut entry) in outcomes {
        entry.rejected.sort_by_key(Value::to_string);
        entry.rejected.dedup();
        if !entry.counts.is_empty() {
            let mut answer = json!({"stage":"parser_storage","counts":entry.counts,"rejected_inputs":entry.rejected});
            if entry.final_values.len() == 1 {
                answer["final_value"] = json!(entry.final_values.first().expect("one value"));
            }
            snapshot.rules.push(Rule {
                id: format!("{id}#occurrences.parser_accepted"),
                subject: id.clone(),
                property: "occurrences.parser_accepted".into(),
                conditions: Vec::new(),
                answer,
                evidence: entry.evidence,
            });
        } else {
            gap(
                snapshot,
                &id,
                "occurrences.parser_accepted",
                if entry.reasons.is_empty() {
                    "No diagnostic-free parser storage outcome was established".into()
                } else {
                    entry
                        .reasons
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                },
                None,
                Vec::new(),
                entry.gap_evidence.clone(),
            );
        }
        for (property, reasons) in entry.reasons {
            gap(
                snapshot,
                &id,
                property,
                reasons.join("; "),
                None,
                Vec::new(),
                entry.gap_evidence.clone(),
            );
        }
    }
    let answered: BTreeSet<_> = snapshot
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .chain(snapshot.gaps.iter().map(|gap| gap.id.as_str()))
        .collect();
    let missing: Vec<_> = snapshot
        .subjects
        .iter()
        .filter(|subject| subject.kind == SubjectKind::Field)
        .filter(|subject| {
            !answered.contains(format!("{}#occurrences.parser_accepted", subject.id).as_str())
        })
        .map(|subject| subject.id.clone())
        .collect();
    for id in missing {
        let evidence = snapshot
            .rules
            .iter()
            .find(|rule| rule.id == format!("{id}#existence"))
            .map(|rule| rule.evidence.clone())
            .unwrap_or_default();
        gap(
            snapshot,
            &id,
            "occurrences.parser_accepted",
            "No established fixture recipe supplies a valid definition and observation phase for this field",
            None,
            Vec::new(),
            evidence,
        );
    }
    Ok(())
}
