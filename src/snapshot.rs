//! Deterministic Atlas rules assembled from Native answers.

use crate::extraction::Extraction;
use pdx_native::{
    Answer, Basis, Completeness, DiagnosticCoverage, DiagnosticJoin, Disposal, Field,
    FixtureFieldOutcome, FixtureObservation, FixtureRuntime, FixtureStorage, Gap as NativeGap,
    ReaderKind, Source, Support,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The versioned offline rule snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Discriminator for Atlas rules.
    pub kind: String,
    /// Version of this envelope and its semantics.
    pub contract_version: u32,
    /// JSON Schema dialect used by the bundled definitions.
    pub schema_dialect: String,
    /// Stable snapshot name and revision.
    pub snapshot: SnapshotIdentity,
    /// Producer identity.
    pub generator: Generator,
    /// Exact Native build identifiers for which these answers apply.
    pub applicability: Applicability,
    /// Declared scope and limits.
    pub coverage: Coverage,
    /// Whole Native source stamps, indexed by stable method and basis.
    pub sources: BTreeMap<String, Source>,
    /// Definitions shared by fields with one reader.
    pub schemas: SchemaBundle,
    /// Registry and field identities.
    pub subjects: Vec<Subject>,
    /// Established answers.
    pub rules: Vec<Rule>,
    /// Properties not established by the evidence.
    pub gaps: Vec<Gap>,
}

/// Stable identity of one Atlas snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotIdentity {
    /// Semantic snapshot family.
    pub name: String,
    /// Revision within this family.
    pub version: u32,
}

/// Atlas producer identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    /// Producing tool.
    pub name: String,
    /// Producing tool version.
    pub version: String,
}

/// Target builds supplied by Native.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applicability {
    /// Exact opaque build identifiers.
    pub builds: Vec<pdx_native::BuildId>,
}

/// Scope explicitly established by this producer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// Content directories requested from Native.
    pub registries: Vec<String>,
    /// Property families this snapshot can establish.
    pub established_properties: Vec<String>,
    /// Always `not_established` for this bounded extraction.
    pub whole_registry_validity: String,
    /// Native's reported operation support.
    pub native_support: BTreeMap<String, Support>,
}

/// JSON Schema definitions linked from value-form rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaBundle {
    /// JSON Schema dialect.
    #[serde(rename = "$schema")]
    pub dialect: String,
    /// Shared reader definitions.
    #[serde(rename = "$defs")]
    pub definitions: BTreeMap<String, Value>,
}

/// A registry or one discovered root field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// Stable subject identity.
    pub id: String,
    /// `registry` or `field`.
    pub kind: String,
    /// Native content directory.
    pub registry: String,
    /// Field name, if this is a field subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Whether the reader depends on state beyond the field key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<bool>,
}

/// An established property for one subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Stable subject and property identity.
    pub id: String,
    /// Subject identity.
    pub subject: String,
    /// Established property.
    pub property: String,
    /// Conditions required by this answer.
    pub conditions: Vec<String>,
    /// Structured answer, not a config assertion.
    pub answer: Value,
    /// Native answers that establish this rule.
    pub evidence: Vec<EvidenceLink>,
}

/// A property whose answer remains unknown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    /// Stable subject and property identity.
    pub id: String,
    /// Subject identity.
    pub subject: String,
    /// Unresolved property.
    pub property: String,
    /// Why evidence is insufficient.
    pub reason: String,
    /// Ticket that owns the missing capability, when known.
    pub owner: Option<String>,
    /// Native's typed gaps, retained whole.
    pub native_gaps: Vec<NativeGap>,
    /// Native answer related to this gap, when available.
    pub evidence: Vec<EvidenceLink>,
}

/// Trace from a rule or gap to a complete Native source stamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLink {
    /// Key in `sources`.
    pub source: String,
    /// Completeness of the Native answer.
    pub completeness: Completeness,
    /// Location within the extraction result.
    pub location: String,
    /// Full typed gap list on the Native answer.
    pub native_gaps: Vec<NativeGap>,
}

const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
/// Assemble a snapshot; malformed internal references or conflicting sources fail.
pub fn assemble(extraction: &Extraction) -> Result<Snapshot, String> {
    if let Err(error) = &extraction.registries {
        return Err(format!("Native registry discovery failed: {error:?}"));
    }
    let mut snapshot = Snapshot {
        kind: "atlas_rule_snapshot".into(),
        contract_version: 1,
        schema_dialect: DIALECT.into(),
        snapshot: SnapshotIdentity {
            name: "stellaris-registry-rules".into(),
            version: 1,
        },
        generator: Generator {
            name: "pdx-atlas".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        applicability: Applicability {
            builds: vec![extraction.build.clone()],
        },
        coverage: Coverage {
            registries: extraction.fields.keys().cloned().collect(),
            established_properties: vec![
                "existence".into(),
                "loader_path".into(),
                "occurrences.parser_accepted".into(),
                "value_form".into(),
            ],
            whole_registry_validity: "not_established".into(),
            native_support: extraction.native_support.clone(),
        },
        sources: BTreeMap::new(),
        schemas: SchemaBundle {
            dialect: DIALECT.into(),
            definitions: BTreeMap::new(),
        },
        subjects: Vec::new(),
        rules: Vec::new(),
        gaps: Vec::new(),
    };
    for registry in extraction.fields.keys() {
        assemble_registry(&mut snapshot, extraction, registry)?;
        assemble_fields(&mut snapshot, extraction, registry)?;
    }
    assemble_outcomes(&mut snapshot, extraction)?;
    snapshot.subjects.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.rules.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.gaps.sort_by(|a, b| a.id.cmp(&b.id));
    let content = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
    snapshot.snapshot.name = format!("stellaris-registry-rules/{:x}", Sha256::digest(content));
    verify(&snapshot)?;
    Ok(snapshot)
}

fn registry_id(registry: &str) -> String {
    format!("registry:{registry}")
}

fn field_id(registry: &str, field: &str) -> String {
    format!("field:{registry}/{field}")
}

fn source_link<T>(
    snapshot: &mut Snapshot,
    answer: &Answer<T>,
    location: String,
) -> Result<EvidenceLink, String> {
    let key = source_key(&answer.source);
    if let Some(existing) = snapshot.sources.get(&key) {
        if existing != &answer.source {
            return Err(format!("Conflicting Native sources under {key}"));
        }
    } else {
        snapshot.sources.insert(key.clone(), answer.source.clone());
    }
    Ok(EvidenceLink {
        source: key,
        completeness: answer.completeness,
        location,
        native_gaps: answer.gaps.clone(),
    })
}

fn source_key(source: &Source) -> String {
    let basis = match source.basis {
        Basis::Declared => "declared",
        Basis::StaticAnalysis => "static_analysis",
        Basis::LiveObservation => "live_observation",
        Basis::Recorded => "recorded",
    };
    format!("{}@{basis}", source.method)
}

fn rule(
    snapshot: &mut Snapshot,
    subject: &str,
    property: &str,
    answer: Value,
    evidence: EvidenceLink,
) {
    snapshot.rules.push(Rule {
        id: format!("{subject}#{property}"),
        subject: subject.into(),
        property: property.into(),
        conditions: Vec::new(),
        answer,
        evidence: vec![evidence],
    });
}

fn gap(
    snapshot: &mut Snapshot,
    subject: &str,
    property: &str,
    reason: impl Into<String>,
    native_gaps: Vec<NativeGap>,
    evidence: Vec<EvidenceLink>,
) {
    snapshot.gaps.push(Gap {
        id: format!("{subject}#{property}"),
        subject: subject.into(),
        property: property.into(),
        reason: reason.into(),
        owner: None,
        native_gaps,
        evidence,
    });
}

fn assemble_registry(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<(), String> {
    let id = registry_id(registry);
    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: "registry".into(),
        registry: registry.into(),
        field: None,
        conditional: None,
    });
    match &extraction.registries {
        Ok(answer) if answer.value.iter().any(|found| found.name == registry) => {
            let evidence = source_link(snapshot, answer, format!("answers.registries:{registry}"))?;
            rule(snapshot, &id, "existence", json!(true), evidence.clone());
            rule(snapshot, &id, "loader_path", json!(registry), evidence);
        }
        Ok(answer) => {
            let evidence = source_link(snapshot, answer, format!("answers.registries:{registry}"))?;
            gap(
                snapshot,
                &id,
                "existence",
                "Native did not name this registry in the bounded answer",
                answer.gaps.clone(),
                vec![evidence.clone()],
            );
            gap(
                snapshot,
                &id,
                "loader_path",
                "Native did not establish this registry's content directory",
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
                Vec::new(),
                Vec::new(),
            );
            return Ok(());
        }
    };
    if answer.completeness == Completeness::Partial {
        let evidence = source_link(snapshot, answer, format!("answers.fields.{registry}"))?;
        gap(
            snapshot,
            &registry_subject,
            "fields_complete",
            "Native's root-field search is partial; undiscovered fields remain unknown",
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
        kind: "field".into(),
        registry: registry.into(),
        field: Some(field.name.clone()),
        conditional: Some(field.conditional),
    });
    let evidence = source_link(
        snapshot,
        answer,
        format!("answers.fields.{registry}:{}", field.name),
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
            answer.gaps.clone(),
            vec![evidence.clone()],
        );
        gap(
            snapshot,
            id,
            "value_form",
            "The reader's conditions are not established",
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
                    Vec::new(),
                    vec![evidence.clone()],
                );
                gap(
                    snapshot,
                    id,
                    "scope_context",
                    "A block reader does not establish scope context",
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
            let evidence = source_link(
                snapshot,
                observation,
                format!(
                    "answers.sessions.{}.fixture:{}:{}",
                    session.name, outcome.question.definition, outcome.question.field
                ),
            )?;
            entry.gap_evidence.push(evidence.clone());
            match classify_parser_outcome(&observation.value, outcome) {
                ParserOutcome::Accepted { count, final_value } => {
                    entry.counts.insert(count);
                    if let Some(final_value) = final_value {
                        entry.final_values.insert(final_value.into());
                    }
                    entry.evidence.push(evidence);
                }
                ParserOutcome::Rejected(rejected) => {
                    entry.rejected.extend(rejected);
                    entry.evidence.push(evidence);
                }
                ParserOutcome::Unavailable { property, reason } => {
                    entry.reasons.entry(property).or_default().push(reason)
                }
            }
            if let FixtureRuntime::Unavailable(reason) = &outcome.runtime {
                let runtime_evidence = source_link(
                    snapshot,
                    observation,
                    format!("answers.sessions.{}.fixture.runtime", session.name),
                )?;
                gap(
                    snapshot,
                    &id,
                    "runtime",
                    reason.clone(),
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
        .filter(|subject| subject.kind == "field")
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
            Vec::new(),
            evidence,
        );
    }
    Ok(())
}

/// Check internal identity, evidence, and schema-reference closure.
pub fn verify(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.kind != "atlas_rule_snapshot" || snapshot.contract_version != 1 {
        return Err("Unsupported rule snapshot contract".into());
    }
    if snapshot.schema_dialect != DIALECT || snapshot.schemas.dialect != DIALECT {
        return Err("Unsupported rule snapshot schema dialect".into());
    }
    if snapshot.coverage.whole_registry_validity != "not_established" {
        return Err("Whole-registry validity was not established".into());
    }
    let [build] = snapshot.applicability.builds.as_slice() else {
        return Err("Snapshot requires one exact Native build".into());
    };
    for (key, source) in &snapshot.sources {
        if &source.build != build {
            return Err("Native source build differs from snapshot applicability".into());
        }
        if *key != source_key(source) {
            return Err(format!("Native source key differs from its stamp: {key}"));
        }
    }
    let mut subjects = BTreeSet::new();
    for subject in &snapshot.subjects {
        let expected = match (subject.kind.as_str(), &subject.field, subject.conditional) {
            ("registry", None, None) => registry_id(&subject.registry),
            ("field", Some(field), Some(_)) => field_id(&subject.registry, field),
            _ => return Err(format!("Invalid subject shape: {}", subject.id)),
        };
        if subject.id != expected || !subjects.insert(subject.id.as_str()) {
            return Err(format!("Invalid or duplicate subject {}", subject.id));
        }
    }
    let mut records = BTreeSet::new();
    for rule in &snapshot.rules {
        if !records.insert(rule.id.as_str())
            || rule.id != format!("{}#{}", rule.subject, rule.property)
            || !subjects.contains(rule.subject.as_str())
        {
            return Err(format!("Invalid rule identity or subject: {}", rule.id));
        }
        if rule.evidence.is_empty() || rule.answer.is_null() {
            return Err(format!("Rule lacks evidence or answer: {}", rule.id));
        }
        verify_evidence(snapshot, &rule.evidence)?;
        if let Some(reference) = rule.answer.get("schema").and_then(Value::as_str) {
            let name = reference
                .strip_prefix("#/schemas/$defs/")
                .ok_or("Invalid schema reference")?;
            if !snapshot.schemas.definitions.contains_key(name) {
                return Err(format!("Missing schema {name}"));
            }
        }
    }
    for gap in &snapshot.gaps {
        if !records.insert(gap.id.as_str())
            || gap.id != format!("{}#{}", gap.subject, gap.property)
            || !subjects.contains(gap.subject.as_str())
            || gap.reason.trim().is_empty()
        {
            return Err(format!(
                "Invalid gap identity, subject, or reason: {}",
                gap.id
            ));
        }
        verify_evidence(snapshot, &gap.evidence)?;
    }
    Ok(())
}

fn verify_evidence(snapshot: &Snapshot, evidence: &[EvidenceLink]) -> Result<(), String> {
    for link in evidence {
        if !snapshot.sources.contains_key(&link.source) || link.location.trim().is_empty() {
            return Err(format!("Missing source or location: {}", link.source));
        }
    }
    Ok(())
}

/// Pretty JSON bytes with a trailing newline.
pub fn json_bytes(snapshot: &Snapshot) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(snapshot)?;
    bytes.push(b'\n');
    Ok(bytes)
}
