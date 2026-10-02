//! Registry, field and fixture-outcome rules.

use super::{
    EvidenceLink, Rule, Snapshot, Subject, SubjectKind, evidence, field_id, gap, registry_id, rule,
};
use crate::extraction::Extraction;
use pdx_native::{
    Answer, BlockFamily, Completeness, DiagnosticCoverage, DiagnosticJoin, Disposal, Field,
    FieldCondition, FieldMembers, FieldReadOutcome, FixtureFieldOutcome, FixtureObservation,
    FixtureStorage, GapSubject, ReaderKind, RepeatBehavior,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Property families that registry assembly can establish.
pub(super) const PROPERTIES: &[&str] = &[
    "existence",
    "loader_path",
    "occurrences.parser_accepted",
    "value_form",
    "read",
    "shape",
    "repeat_behavior",
    "conditions",
    "block_family",
    "members",
    "uses",
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
                Some(GapSubject::Registry {
                    name: registry.into(),
                }),
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
                Some(GapSubject::Registry {
                    name: registry.into(),
                }),
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
                    format!(
                        "Native registry question failed: {}",
                        super::failure::error_reason(error)
                    ),
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
                format!(
                    "Native field question failed: {}",
                    super::failure::error_reason(error)
                ),
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
    let mut paths = BTreeMap::new();
    field_paths(&answer.value, "", &mut paths);
    let unjoined: Vec<_> = answer
        .gaps
        .iter()
        .filter(|gap| match &gap.subject {
            Some(GapSubject::Field { name }) => {
                paths.get(name).is_none_or(|paths| paths.len() != 1)
            }
            _ => false,
        })
        .cloned()
        .collect();
    if !unjoined.is_empty() {
        let names: BTreeSet<_> = unjoined
            .iter()
            .filter_map(|gap| match &gap.subject {
                Some(GapSubject::Field { name }) => Some(format!(
                    "{name}: {}",
                    paths
                        .get(name)
                        .map(|paths| paths.join(", "))
                        .unwrap_or_else(|| "no discovered field".into())
                )),
                _ => None,
            })
            .collect();
        let link = evidence(
            snapshot,
            &format!("registry_fields/{registry}"),
            answer,
            format!("answers.fields.{registry}"),
            None,
        )?;
        gap(
            snapshot,
            &registry_subject,
            "field_gap_subjects",
            format!(
                "Native field-gap names do not identify one field path: {}",
                names.into_iter().collect::<Vec<_>>().join("; ")
            ),
            None,
            unjoined,
            vec![link],
        );
    }
    for field in &answer.value {
        assemble_field(
            snapshot,
            registry,
            &field.name,
            field,
            answer,
            false,
            &paths,
        )?;
    }
    Ok(())
}

fn field_paths(fields: &[Field], parent: &str, paths: &mut BTreeMap<String, Vec<String>>) {
    for field in fields {
        let path = if parent.is_empty() {
            field.name.clone()
        } else {
            format!("{parent}/{}", field.name)
        };
        paths
            .entry(field.name.clone())
            .or_default()
            .push(path.clone());
        if let FieldMembers::Fields(children) = &field.members {
            field_paths(children, &path, paths);
        }
    }
}

fn assemble_field(
    snapshot: &mut Snapshot,
    registry: &str,
    path: &str,
    field: &Field,
    answer: &Answer<Vec<Field>>,
    parent_conditional: bool,
    paths: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let unconditional_outcome = match field.read.as_slice() {
        [alternative] if !parent_conditional && alternative.condition == FieldCondition::Always => {
            Some(&alternative.outcome)
        }
        _ => None,
    };
    let conditional = unconditional_outcome.is_none();
    let id = field_id(registry, path);
    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: SubjectKind::Field,
        registry: Some(registry.into()),
        field: Some(path.into()),
        conditional: Some(conditional),
        name: None,
    });
    let evidence = evidence(
        snapshot,
        &format!("registry_fields/{registry}"),
        answer,
        format!("answers.fields.{registry}:{path}"),
        paths
            .get(&field.name)
            .filter(|paths| paths.as_slice() == [path])
            .map(|_| GapSubject::Field {
                name: field.name.clone(),
            }),
    )?;
    rule(snapshot, &id, "existence", json!(true), evidence.clone());
    if field.read.is_empty() {
        return Err(format!("Field {id} has no read alternatives"));
    }
    rule(snapshot, &id, "read", json!(field.read), evidence.clone());
    rule(snapshot, &id, "shape", json!(field.shape), evidence.clone());
    rule(
        snapshot,
        &id,
        "conditions",
        json!(
            field
                .read
                .iter()
                .map(|alternative| &alternative.condition)
                .collect::<Vec<_>>()
        ),
        evidence.clone(),
    );
    rule(
        snapshot,
        &id,
        "uses",
        json!({"stage":"stored_value_selection", "selections":field.uses}),
        evidence.clone(),
    );
    rule(
        snapshot,
        &id,
        "block_family",
        json!(
            field
                .read
                .iter()
                .map(|alternative| {
                    let family = match &alternative.outcome {
                        FieldReadOutcome::Read { reader, .. } => reader.family,
                        _ => BlockFamily::Unknown,
                    };
                    json!({"condition":alternative.condition, "family":family})
                })
                .collect::<Vec<_>>()
        ),
        evidence.clone(),
    );
    gap(
        snapshot,
        &id,
        "domain",
        "Native reports this field property as Unknown",
        Some("field_semantics"),
        Vec::new(),
        vec![evidence.clone()],
    );
    rule(
        snapshot,
        &id,
        "members",
        json!(field.members),
        evidence.clone(),
    );
    if let FieldMembers::Fields(children) = &field.members {
        for child in children {
            assemble_field(
                snapshot,
                registry,
                &format!("{path}/{}", child.name),
                child,
                answer,
                conditional,
                paths,
            )?;
        }
    }
    assemble_field_value_form(snapshot, &id, field, &evidence, unconditional_outcome)?;
    assemble_field_repeat(snapshot, &id, unconditional_outcome, &evidence);
    assemble_field_gaps(snapshot, &id, field, &evidence);
    Ok(())
}

fn assemble_field_value_form(
    snapshot: &mut Snapshot,
    id: &str,
    field: &Field,
    evidence: &EvidenceLink,
    outcome: Option<&FieldReadOutcome>,
) -> Result<(), String> {
    let (reader, reason, owner) = match outcome {
        Some(FieldReadOutcome::Read { reader, .. }) => (Some(reader), "", None),
        Some(FieldReadOutcome::Rejected) => (
            None,
            "Native rejects this field on the unconditional loader path",
            None,
        ),
        Some(_) => (
            None,
            "Native did not establish the unconditional reader's value form",
            None,
        ),
        None => (
            None,
            "Conditional alternatives do not establish an unconditional value form; unresolved branches remain unknown",
            Some("field_conditions"),
        ),
    };
    let Some(reader) = reader else {
        rule(
            snapshot,
            id,
            "value_form",
            json!({"alternatives":field.read}),
            evidence.clone(),
        );
        gap(
            snapshot,
            id,
            "value_form.unresolved",
            reason,
            owner,
            evidence.native_gaps.clone(),
            vec![evidence.clone()],
        );
        return Ok(());
    };
    let reader_id = &reader.id;
    let reader_id = reader_id
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| error.to_string())?;
    let shape = match reader.kind {
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
        let schema_name = format!(
            "reader-{}",
            reader_id.as_ref().and_then(Value::as_str).unwrap_or(form)
        );
        let schema = json!({"type":schema_type});
        if let Some(existing) = snapshot.schemas.definitions.get(&schema_name) {
            if existing != &schema {
                return Err(format!("Conflicting shapes for reader {reader_id:?}"));
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
    } else if reader.kind != ReaderKind::Block {
        gap(
            snapshot,
            id,
            "value_form",
            "Native did not establish this reader's value form",
            None,
            evidence.native_gaps.clone(),
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
    gap(
        snapshot,
        id,
        "occurrences.minimum",
        "No validation result establishes that this field is required",
        None,
        Vec::new(),
        vec![evidence.clone()],
    );
}

fn assemble_field_repeat(
    snapshot: &mut Snapshot,
    id: &str,
    outcome: Option<&FieldReadOutcome>,
    evidence: &EvidenceLink,
) {
    match outcome {
        Some(FieldReadOutcome::Read { shape, .. })
            if matches!(
                shape.repeat,
                RepeatBehavior::Replace | RepeatBehavior::Accumulate
            ) =>
        {
            rule(
                snapshot,
                id,
                "repeat_behavior",
                json!(shape.repeat),
                evidence.clone(),
            );
        }
        _ => gap(
            snapshot,
            id,
            "occurrences.maximum",
            "Native did not establish unconditional repeat behavior",
            None,
            Vec::new(),
            vec![evidence.clone()],
        ),
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
        FixtureStorage::Observed {
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
        FixtureStorage::Observed {
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
                    "Native fixture session disposal failed: {}",
                    super::failure::session_reason(&session.disposal)
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
                    format!(
                        "Native fixture question failed: {}",
                        super::failure::error_reason(error)
                    ),
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
