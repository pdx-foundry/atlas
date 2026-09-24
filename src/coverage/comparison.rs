//! Test-only comparison of projected Atlas answers with CWT claims.

mod name_lists;
pub mod script_docs;

use super::{Answer, Gap, projection};
use crate::{ledger::Ledger, snapshot};
use pdx_native::LoadedModifiers;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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

/// Names or entries that the engine and a config source agree on, and those only one gives.
#[derive(Clone, Debug, Serialize)]
pub struct NameList {
    /// What the list holds, such as `effects` or `descriptions`.
    pub list: String,
    /// Entries that both give.
    pub agree: Vec<String>,
    /// Entries that only the snapshot or its Native answer gives.
    pub engine_only: Vec<String>,
    /// Entries that only the config source gives.
    pub config_only: Vec<String>,
}

/// One `script-docs` log compared with the snapshot.
#[derive(Clone, Debug, Serialize)]
pub struct LogComparison {
    /// Log file name, such as `effects.log`.
    pub log: String,
    /// Names, then each property as `name: value` entries over the names that both answer.
    pub lists: Vec<NameList>,
}

/// Static category tags of declared modifiers against their tags in the loaded table.
#[derive(Clone, Debug, Serialize)]
pub struct TagComparison {
    /// Number of declared modifiers whose loaded tags equal their static tags.
    pub agree: usize,
    /// Declared modifiers whose loaded tags differ or are not established.
    pub different: Vec<TagDifference>,
}

/// One declared modifier whose loaded tags differ from its static tags.
#[derive(Clone, Debug, Serialize)]
pub struct TagDifference {
    /// Modifier name.
    pub name: String,
    /// Tags that the executable declares, or `None` when they are not established.
    pub declared: Option<Vec<String>>,
    /// Tags in the loaded table, or `None` when they are not established or the name is absent.
    pub loaded: Option<Vec<String>>,
}

/// Evidence besides the snapshot that the comparison reads. Each part is optional.
#[derive(Default)]
pub struct Inputs<'a> {
    /// `script-docs` logs by file name; see [`script_docs::LOGS`].
    pub script_docs: BTreeMap<String, String>,
    /// The installation's define files, keyed by path.
    pub define_files: BTreeMap<String, String>,
    /// Native's loaded modifier answer, recorded with the snapshot for the same build.
    pub loaded_modifiers: Option<&'a pdx_native::Answer<LoadedModifiers>>,
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
    /// Each config name list against the snapshot. Modifiers include loaded names when read.
    pub name_lists: Vec<NameList>,
    /// Each supplied `script-docs` log against the snapshot.
    pub script_docs: Vec<LogComparison>,
    /// Define names of the supplied define files against the snapshot, when supplied.
    pub define_files: Option<NameList>,
    /// Whether Native's loaded modifier answer was read.
    pub loaded_modifiers_read: bool,
    /// Loaded against static modifier tags, when the loaded answer was read.
    pub modifier_tags: Option<TagComparison>,
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
            "type_existence"
            | "field_existence"
            | "command_existence"
            | "declaration_existence" => Some(Self::Presence),
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

type QuestionIndex<'a, T> = BTreeMap<(&'a str, &'a [String]), Vec<&'a T>>;

fn question_index<'a, T>(
    items: &'a [T],
    key: impl Fn(&'a T) -> (&'a str, &'a [String]),
) -> QuestionIndex<'a, T> {
    let mut index = QuestionIndex::new();
    for item in items {
        index.entry(key(item)).or_default().push(item);
    }
    index
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

fn entry(
    claim: &crate::ledger::Claim,
    answers_by_question: &QuestionIndex<'_, Answer>,
    gaps_by_question: &QuestionIndex<'_, Gap>,
) -> Entry {
    let key = (claim.question.as_str(), claim.conditions.as_slice());
    let answers = answers_by_question
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let gaps = gaps_by_question
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|gap| gap.reason.clone())
        .collect::<Vec<_>>();
    let atlas_answers = answers
        .iter()
        .map(|answer| answer.value.clone())
        .collect::<Vec<_>>();
    let (status, reason) = comparison_outcome(claim, answers, &gaps);
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

/// Compares an Atlas rule snapshot directly with the config ledger and the supplied inputs.
/// Recorded answers remain comparable, but this report grants no coverage credit.
pub fn evaluate(ledger: &Ledger, input: &[u8], inputs: &Inputs) -> Result<Report, String> {
    let rules: snapshot::Snapshot =
        serde_json::from_slice(input).map_err(|error| format!("Invalid rule snapshot: {error}"))?;
    if let Some(loaded) = inputs.loaded_modifiers
        && rules.applicability.builds != [loaded.source.build.clone()]
    {
        return Err("The loaded modifier answer is for another build than the snapshot".into());
    }
    let projection = projection::project_for_comparison(ledger, &rules)?;
    let answers_by_question = question_index(&projection.answers, |answer: &Answer| {
        (&answer.question, &answer.conditions)
    });
    let gaps_by_question = question_index(&projection.gaps, |gap: &Gap| {
        (&gap.question, &gap.conditions)
    });
    let mut entries = ledger
        .claims
        .iter()
        .map(|claim| entry(claim, &answers_by_question, &gaps_by_question))
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
            atlas_answers: answers_by_question
                .get(&(question.as_str(), conditions.as_slice()))
                .into_iter()
                .flatten()
                .map(|answer| answer.value.clone())
                .collect(),
            gaps: gaps_by_question
                .get(&(question.as_str(), conditions.as_slice()))
                .into_iter()
                .flatten()
                .map(|gap| gap.reason.clone())
                .collect(),
            reason: None,
        });
    }
    let engine = name_lists::Engine::new(&rules);

    Ok(Report {
        format_version: 2,
        config_sha256: ledger.config_sha256.clone(),
        snapshot_sha256: format!("{:x}", Sha256::digest(input)),
        snapshot_id: projection.snapshot_id,
        inventory_complete: ledger.diagnostics.is_empty(),
        entries,
        name_lists: name_lists::config_lists(ledger, &engine, inputs),
        script_docs: script_docs::compare_logs(&engine, inputs),
        define_files: name_lists::define_files(&engine, inputs)?,
        loaded_modifiers_read: inputs.loaded_modifiers.is_some(),
        modifier_tags: name_lists::modifier_tags(&engine, inputs),
    })
}
