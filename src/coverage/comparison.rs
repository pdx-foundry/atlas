//! Test-only comparison of projected Atlas answers with CWT claims.

use super::{Answer, Gap, Projection, projection};
use crate::{ledger::Ledger, snapshot};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Relationship between one config claim and an Atlas answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The normalized answers agree.
    Same,
    /// The normalized answers disagree.
    Different,
    /// No comparable Atlas answer is available for this config claim.
    MissingFromAtlas,
    /// The Atlas question has no matching config claim.
    AtlasOnly,
}

/// One source claim or Atlas-only question, with both sides visible.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// Config occurrence ID, absent for Atlas-only questions.
    pub claim: Option<String>,
    /// Shared question ID or Atlas-only rule ID.
    pub question: String,
    /// Config source file, absent for Atlas-only questions.
    pub file: Option<String>,
    /// Structural applicability conditions.
    pub conditions: Vec<String>,
    /// Property being compared, absent for Atlas-only questions.
    pub property: Option<String>,
    /// Comparison result.
    pub status: Status,
    /// The config assertion, absent for Atlas-only questions.
    pub config_answer: Option<String>,
    /// Applicable structured Atlas answers, including incomplete or conflicting answers.
    pub atlas_answers: Vec<Value>,
    /// Reasons for applicable Atlas gaps.
    pub gaps: Vec<String>,
    /// Explanation when no comparison can be made.
    pub reason: Option<String>,
}

/// Deterministic comparison report, separate from the coverage score.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// Report contract version.
    pub format_version: u32,
    /// Exact config input identity.
    pub config_sha256: String,
    /// Exact snapshot input identity.
    pub snapshot_sha256: String,
    /// Snapshot name and revision.
    pub snapshot_id: String,
    /// Whether the config inventory has unresolved diagnostics.
    pub inventory_complete: bool,
    /// Config claims followed by Atlas-only questions, in stable order.
    pub entries: Vec<Entry>,
}

enum ComparisonKind {
    Presence,
    LoaderPath,
    ValueForm,
    Cardinality,
}

impl ComparisonKind {
    fn for_property(property: &str) -> Option<Self> {
        match property {
            "type_existence" | "field_existence" => Some(Self::Presence),
            "loader_path" => Some(Self::LoaderPath),
            "value_form" => Some(Self::ValueForm),
            "cardinality_minimum" | "cardinality_maximum" => Some(Self::Cardinality),
            _ => None,
        }
    }

    fn config_value(&self, raw: &str) -> Option<Value> {
        match self {
            Self::Presence => match raw {
                "present" => Some(Value::Bool(true)),
                "absent" => Some(Value::Bool(false)),
                _ => None,
            },
            Self::LoaderPath => raw
                .trim_matches('"')
                .strip_prefix("game/")
                .map(|path| Value::String(path.into())),
            Self::ValueForm => ["bool", "int", "float", "string", "scalar", "block"]
                .contains(&raw)
                .then(|| Value::String(raw.into())),
            Self::Cardinality => raw.parse::<u64>().ok().map(Value::from),
        }
    }

    fn atlas_value(&self, answer: &Value) -> Option<Value> {
        match self {
            Self::Presence => answer.as_bool().map(Value::Bool),
            Self::LoaderPath => answer.as_str().map(|path| Value::String(path.into())),
            Self::ValueForm => answer.get("form")?.as_str().map(|form| {
                Value::String(
                    match form {
                        "boolean" => "bool",
                        "integer" => "int",
                        "number" => "float",
                        other => other,
                    }
                    .into(),
                )
            }),
            Self::Cardinality => answer.as_u64().map(Value::from),
        }
    }

    fn agrees(&self, config: &Value, atlas: &Value) -> bool {
        if config == atlas {
            return true;
        }
        matches!(self, Self::ValueForm)
            && config.as_str() == Some("scalar")
            && matches!(
                atlas.as_str(),
                Some("bool" | "int" | "float" | "string" | "reference")
            )
    }
}

fn applicable<'a, T>(
    items: &'a [T],
    question: &str,
    conditions: &[String],
    key: impl Fn(&'a T) -> (&'a str, &'a [String]),
) -> Vec<&'a T> {
    items
        .iter()
        .filter(|item| {
            let (item_question, item_conditions) = key(item);
            item_question == question && item_conditions == conditions
        })
        .collect()
}

fn comparison_outcome(
    claim: &crate::ledger::Claim,
    answers: &[&Answer],
    gaps: &[String],
) -> (Status, Option<String>) {
    if !gaps.is_empty() {
        return (
            Status::MissingFromAtlas,
            Some("An applicable Atlas gap prevents comparison".into()),
        );
    }
    if answers.is_empty() {
        return (
            Status::MissingFromAtlas,
            Some("No applicable Atlas answer".into()),
        );
    }
    let Some(kind) = ComparisonKind::for_property(&claim.property) else {
        return (
            Status::MissingFromAtlas,
            Some("Config assertion has no comparison mapping for this property".into()),
        );
    };
    let Some(config) = kind.config_value(&claim.config_answer) else {
        return (
            Status::MissingFromAtlas,
            Some("Config assertion has no comparable form for this property".into()),
        );
    };
    let normalized = answers
        .iter()
        .map(|answer| kind.atlas_value(&answer.value))
        .collect::<Option<Vec<_>>>();
    match normalized {
        Some(values) if values.iter().all(|value| kind.agrees(&config, value)) => {
            (Status::Same, None)
        }
        Some(_) => (Status::Different, None),
        None => (
            Status::MissingFromAtlas,
            Some("Atlas answer has no comparable form for this property".into()),
        ),
    }
}

fn entry(claim: &crate::ledger::Claim, projection: &Projection) -> Entry {
    let answers = applicable(
        &projection.answers,
        &claim.question,
        &claim.conditions,
        |answer: &Answer| (&answer.question, &answer.conditions),
    );
    let gaps = applicable(
        &projection.gaps,
        &claim.question,
        &claim.conditions,
        |gap: &Gap| (&gap.question, &gap.conditions),
    )
    .into_iter()
    .map(|gap| gap.reason.clone())
    .collect::<Vec<_>>();
    let atlas_answers = answers
        .iter()
        .map(|answer| answer.value.clone())
        .collect::<Vec<_>>();
    let (status, reason) = comparison_outcome(claim, &answers, &gaps);
    Entry {
        claim: Some(claim.id.clone()),
        question: claim.question.clone(),
        file: Some(claim.file.clone()),
        conditions: claim.conditions.clone(),
        property: Some(claim.property.clone()),
        status,
        config_answer: Some(claim.config_answer.clone()),
        atlas_answers,
        gaps,
        reason,
    }
}

/// Compares an Atlas rule snapshot directly with the config ledger.
/// Recorded answers remain comparable, but this report grants no coverage credit.
pub fn evaluate(ledger: &Ledger, input: &[u8]) -> Result<Report, String> {
    let rules: snapshot::Snapshot =
        serde_json::from_slice(input).map_err(|error| format!("Invalid rule snapshot: {error}"))?;
    let projection = projection::project_for_comparison(ledger, &rules)?;
    let mut entries = ledger
        .claims
        .iter()
        .map(|claim| entry(claim, &projection))
        .collect::<Vec<_>>();
    let config_questions = ledger
        .claims
        .iter()
        .map(|claim| (&claim.question, &claim.conditions))
        .collect::<BTreeSet<_>>();
    let atlas_questions = projection
        .answers
        .iter()
        .map(|answer| (&answer.question, &answer.conditions))
        .chain(
            projection
                .gaps
                .iter()
                .map(|gap| (&gap.question, &gap.conditions)),
        )
        .filter(|pair| !config_questions.contains(pair))
        .collect::<BTreeSet<_>>();
    for (question, conditions) in atlas_questions {
        entries.push(Entry {
            claim: None,
            question: question.clone(),
            file: None,
            conditions: conditions.clone(),
            property: None,
            status: Status::AtlasOnly,
            config_answer: None,
            atlas_answers: applicable(
                &projection.answers,
                question,
                conditions,
                |answer: &Answer| (&answer.question, &answer.conditions),
            )
            .into_iter()
            .map(|answer| answer.value.clone())
            .collect(),
            gaps: applicable(&projection.gaps, question, conditions, |gap: &Gap| {
                (&gap.question, &gap.conditions)
            })
            .into_iter()
            .map(|gap| gap.reason.clone())
            .collect(),
            reason: None,
        });
    }
    Ok(Report {
        format_version: 1,
        config_sha256: ledger.config_sha256.clone(),
        snapshot_sha256: format!("{:x}", Sha256::digest(input)),
        snapshot_id: projection.snapshot_id,
        inventory_complete: ledger.diagnostics.is_empty(),
        entries,
    })
}
