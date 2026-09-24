//! Deterministic Atlas rules assembled from Native answers.

mod language;
mod registry;

pub(crate) use language::loaded_summary;

use crate::extraction::Extraction;
use pdx_native::{Answer, Basis, Completeness, Gap as NativeGap, Source, Support};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The contract version that this Atlas writes and reads.
pub const CONTRACT_VERSION: u32 = 2;

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
    /// Each Native answer used as evidence: its source, completeness and typed gaps.
    pub answers: BTreeMap<String, AnswerRecord>,
    /// Definitions shared by fields with one reader.
    pub schemas: SchemaBundle,
    /// Registry, field and language declaration identities.
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

/// One Native answer, stated once and referenced by evidence links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerRecord {
    /// Key in `sources`.
    pub source: String,
    /// Whether Native's stated search completed.
    pub completeness: Completeness,
    /// Every typed gap of the answer.
    pub native_gaps: Vec<NativeGap>,
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

/// What a subject identifies. The kind is also the prefix of the subject identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    /// A content directory.
    Registry,
    /// A root field of a registry's definitions.
    Field,
    /// An effect command.
    Effect,
    /// A trigger command.
    Trigger,
    /// A modifier that the executable declares directly.
    Modifier,
    /// A declared modifier category tag.
    ModifierCategory,
    /// Modifiers that a registry generates for each item, named `{registry}/{template}`.
    ModifierFamily,
    /// A scope type, named `{display name}/{keywords joined by ","}`.
    Scope,
    /// A keyword that matches several scope types.
    ScopeGroup,
    /// A scope link.
    ScopeLink,
    /// A localization context.
    LocalizationContext,
    /// A localization command.
    LocalizationCommand,
    /// A localization link.
    LocalizationLink,
    /// An on_action that the engine fires by name.
    OnAction,
    /// A game rule that the engine evaluates.
    GameRule,
    /// A define, named `{namespace}.{name}`.
    Define,
    /// One Native question as a whole, such as `effects`.
    Inventory,
}

impl SubjectKind {
    /// The identity of the language subject of this kind with this name.
    ///
    /// Registry and field subjects have their own identity rules; see [`Subject`].
    pub fn id(self, name: &str) -> String {
        let prefix = serde_json::to_value(self).expect("unit variant serializes");
        format!("{}:{name}", prefix.as_str().expect("snake-case name"))
    }

    fn is_language(self) -> bool {
        !matches!(self, Self::Registry | Self::Field)
    }
}

/// A registry, one discovered root field, or one language declaration.
///
/// Registry subjects are `registry:{registry}`, field subjects `field:{registry}/{field}`, and
/// every other subject is `{kind}:{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// Stable subject identity.
    pub id: String,
    /// What the subject identifies.
    pub kind: SubjectKind,
    /// Native content directory of a registry or field subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// Field name, if this is a field subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Whether the reader depends on state beyond the field key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditional: Option<bool>,
    /// Name of a language subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
    /// Native's typed gaps behind this gap, retained whole.
    pub native_gaps: Vec<NativeGap>,
    /// Native answer related to this gap, when available.
    pub evidence: Vec<EvidenceLink>,
}

/// Trace from a rule or gap to a Native answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLink {
    /// Key in `answers`.
    pub answer: String,
    /// Location within that answer.
    pub location: String,
    /// The answer's typed gaps that name this location's subject.
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
        contract_version: CONTRACT_VERSION,
        schema_dialect: DIALECT.into(),
        snapshot: SnapshotIdentity {
            name: "stellaris-rules".into(),
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
            established_properties: registry::PROPERTIES
                .iter()
                .chain(language::PROPERTIES)
                .map(|property| (*property).into())
                .collect(),
            whole_registry_validity: "not_established".into(),
            native_support: extraction.native_support.clone(),
        },
        sources: BTreeMap::new(),
        answers: BTreeMap::new(),
        schemas: SchemaBundle {
            dialect: DIALECT.into(),
            definitions: BTreeMap::new(),
        },
        subjects: Vec::new(),
        rules: Vec::new(),
        gaps: Vec::new(),
    };
    registry::assemble(&mut snapshot, extraction)?;
    language::assemble(&mut snapshot, extraction)?;
    snapshot.coverage.established_properties.sort();
    snapshot.coverage.established_properties.dedup();
    snapshot.subjects.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.rules.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.gaps.sort_by(|a, b| a.id.cmp(&b.id));
    let content = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
    snapshot.snapshot.name = format!("stellaris-rules/{:x}", Sha256::digest(content));
    verify(&snapshot)?;
    Ok(snapshot)
}

fn registry_id(registry: &str) -> String {
    format!("registry:{registry}")
}

fn field_id(registry: &str, field: &str) -> String {
    format!("field:{registry}/{field}")
}

/// Record `answer` under `key` and link to one location in it. `item` selects the answer's gaps
/// whose subject is that item.
fn evidence<T>(
    snapshot: &mut Snapshot,
    key: &str,
    answer: &Answer<T>,
    location: String,
    item: Option<&str>,
) -> Result<EvidenceLink, String> {
    let source = source_key(&answer.source);
    match snapshot.sources.get(&source) {
        Some(existing) if existing != &answer.source => {
            return Err(format!("Conflicting Native sources under {source}"));
        }
        Some(_) => {}
        None => {
            snapshot
                .sources
                .insert(source.clone(), answer.source.clone());
        }
    }

    let record = AnswerRecord {
        source,
        completeness: answer.completeness,
        native_gaps: answer.gaps.clone(),
    };
    match snapshot.answers.get(key) {
        Some(existing) if existing != &record => {
            return Err(format!("Conflicting Native answers under {key}"));
        }
        Some(_) => {}
        None => {
            snapshot.answers.insert(key.into(), record);
        }
    }

    let native_gaps = match item {
        Some(item) => answer
            .gaps
            .iter()
            .filter(|gap| gap.subject.as_deref() == Some(item))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    Ok(EvidenceLink {
        answer: key.into(),
        location,
        native_gaps,
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
    conditional_rule(snapshot, subject, property, Vec::new(), answer, evidence);
}

fn conditional_rule(
    snapshot: &mut Snapshot,
    subject: &str,
    property: &str,
    conditions: Vec<String>,
    answer: Value,
    evidence: EvidenceLink,
) {
    snapshot.rules.push(Rule {
        id: format!("{subject}#{property}"),
        subject: subject.into(),
        property: property.into(),
        conditions,
        answer,
        evidence: vec![evidence],
    });
}

fn gap(
    snapshot: &mut Snapshot,
    subject: &str,
    property: &str,
    reason: impl Into<String>,
    owner: Option<&str>,
    native_gaps: Vec<NativeGap>,
    evidence: Vec<EvidenceLink>,
) {
    snapshot.gaps.push(Gap {
        id: format!("{subject}#{property}"),
        subject: subject.into(),
        property: property.into(),
        reason: reason.into(),
        owner: owner.map(Into::into),
        native_gaps,
        evidence,
    });
}

/// Check internal identity, evidence, and schema-reference closure.
pub fn verify(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.kind != "atlas_rule_snapshot" || snapshot.contract_version != CONTRACT_VERSION {
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
    for (key, answer) in &snapshot.answers {
        if !snapshot.sources.contains_key(&answer.source) {
            return Err(format!("Native answer {key} has no source"));
        }
    }

    let mut subjects = BTreeSet::new();
    for subject in &snapshot.subjects {
        let expected = subject_identity(subject)?;
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

fn subject_identity(subject: &Subject) -> Result<String, String> {
    let invalid = || format!("Invalid subject shape: {}", subject.id);
    match (
        subject.kind,
        &subject.registry,
        &subject.field,
        subject.conditional,
        &subject.name,
    ) {
        (SubjectKind::Registry, Some(registry), None, None, None) => Ok(registry_id(registry)),
        (SubjectKind::Field, Some(registry), Some(field), Some(_), None) => {
            Ok(field_id(registry, field))
        }
        (kind, None, None, None, Some(name)) if kind.is_language() && !name.is_empty() => {
            Ok(kind.id(name))
        }
        _ => Err(invalid()),
    }
}

fn verify_evidence(snapshot: &Snapshot, evidence: &[EvidenceLink]) -> Result<(), String> {
    for link in evidence {
        if !snapshot.answers.contains_key(&link.answer) || link.location.trim().is_empty() {
            return Err(format!("Missing answer or location: {}", link.answer));
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
