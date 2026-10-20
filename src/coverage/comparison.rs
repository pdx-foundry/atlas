//! Test-only comparison of projected Atlas answers with CWT claims.

mod name_lists;
pub mod script_docs;

pub use crate::callback_checks::{AgreementCount, CheckOutcome, EntryScopeCheck, EntryScopeChecks};

use super::{Answer, Gap, projection};
use crate::{ledger::Ledger, snapshot};
use pdx_native::LoadedModifiers;
use serde::Serialize;
use serde_json::{Value, json};
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
    /// SDK-608 historical hand checks, only when both callback answers match their exact
    /// build and callback method. Absent or mismatched answers omit the entire check ledger.
    /// These do not compare the current config or change comparison entry statuses.
    pub entry_scope_checks: Option<&'static EntryScopeChecks>,
}

enum ComparisonKind {
    Presence,
    LoaderPath,
    /// `path_file`: the file name against the last segment of a single-file registry's name.
    SingleFile,
    /// `path_extension` against the loader's file-name suffix.
    Extension,
    /// `path_strict`: only the folder's own files against the loader's recursion.
    FolderOnly,
    /// `name_field` against what names the type's definitions.
    NameField,
    /// `skip_root_key`: definitions below a root statement against a nested form.
    SkippedRootKey,
    /// `type_per_file` and `name_from_file`: Native states no form that a file names.
    FileNamed,
    /// An `overrides.cwt` priority against the duplicate rule.
    Priority,
    /// An `overrides.cwt` directory against the presence of a duplicate rule.
    PriorityListed,
    ValueForm,
    Cardinality,
    /// An `enum[X]` member, which the config lists as accepted, against whether a field that
    /// takes its value from the enum accepts it.
    EnumMember,
    /// A subtype's `push_scope` against the scopes that the root statement it selects is read in
    /// (an event kind). The config names a scope type or group by a `scopes.cwt` alias, and `any`
    /// agrees with every answer, including no scope.
    PushScope,
}

impl ComparisonKind {
    fn for_claim(claim: &crate::ledger::Claim) -> Option<Self> {
        if claim.file == "overrides.cwt" {
            return match claim.property.as_str() {
                "value_form" => Some(Self::Priority),
                "field_existence" => Some(Self::PriorityListed),
                _ => None,
            };
        }

        match claim.property.as_str() {
            "type_existence"
            | "field_existence"
            | "command_existence"
            | "declaration_existence" => Some(Self::Presence),
            "loader_path" => Self::for_loader_line(claim.subject.last()?),
            "value_form" => Some(Self::ValueForm),
            "cardinality_minimum" | "cardinality_maximum" => Some(Self::Cardinality),
            "enum_member" => Some(Self::EnumMember),
            "scope_context"
                if matches!(
                    claim.subject.as_slice(),
                    [types, _, subtype, annotation]
                        if types == "types"
                            && super::config_types::is_subtype(subtype)
                            && annotation == "$annotation:push_scope"
                ) =>
            {
                Some(Self::PushScope)
            }
            _ => None,
        }
    }

    fn for_loader_line(line: &str) -> Option<Self> {
        match line {
            "path" => Some(Self::LoaderPath),
            "path_file" => Some(Self::SingleFile),
            "path_extension" => Some(Self::Extension),
            "path_strict" => Some(Self::FolderOnly),
            "name_field" => Some(Self::NameField),
            "skip_root_key" => Some(Self::SkippedRootKey),
            "type_per_file" | "name_from_file" => Some(Self::FileNamed),
            _ => None,
        }
    }

    /// The comparable form of a config assertion. `scopes` holds the scope subject IDs that each
    /// `scopes.cwt` alias names.
    fn config_value(
        &self,
        raw: &str,
        scopes: &BTreeMap<String, BTreeSet<String>>,
    ) -> Option<Value> {
        let text = raw.trim_matches('"');
        let yes_no = || match text {
            "yes" => Some(Value::Bool(true)),
            "no" => Some(Value::Bool(false)),
            _ => None,
        };

        match self {
            Self::Presence => match raw {
                "present" => Some(Value::Bool(true)),
                "absent" => Some(Value::Bool(false)),
                _ => None,
            },
            Self::LoaderPath => Some(Value::String(
                super::config_types::config_directory(raw).into(),
            )),
            Self::SingleFile | Self::Extension => Some(Value::String(text.into())),
            Self::FolderOnly | Self::FileNamed => yes_no(),
            Self::NameField => match text {
                "" => None,
                "-" => Some(json!("Value")),
                field => Some(json!({ "Field": field })),
            },
            Self::SkippedRootKey => Some(Value::Bool(!text.is_empty())),
            Self::Priority => Some(Value::String(
                match text {
                    "lios" => "LastWins",
                    "ordered" => "Merges",
                    other => other,
                }
                .into(),
            )),
            Self::PriorityListed => (text == "present").then_some(Value::Bool(true)),
            Self::ValueForm => ["bool", "int", "float", "string", "scalar", "block"]
                .contains(&raw)
                .then(|| Value::String(raw.into())),
            Self::Cardinality if raw == "inf" => Some(Value::String("inf".into())),
            Self::Cardinality => raw.parse::<u64>().ok().map(Value::from),
            Self::EnumMember => Some(Value::Bool(true)),
            Self::PushScope => match raw.strip_prefix("push_scope:")?.trim() {
                "any" => Some(json!("any")),
                alias => scopes.get(alias).map(|types| json!(types)),
            },
        }
    }

    fn atlas_value(&self, answer: &Value) -> Option<Value> {
        match self {
            Self::Presence => answer.as_bool().map(Value::Bool),
            Self::LoaderPath => answer.as_str().map(|path| Value::String(path.into())),
            Self::SingleFile => answer
                .as_str()
                .and_then(|path| path.rsplit('/').next())
                .map(|file| Value::String(file.into())),
            Self::Extension => answer.get("suffix").cloned(),
            Self::FolderOnly => match answer {
                Value::String(recursion) => Some(Value::Bool(recursion == "None")),
                Value::Object(levels) if levels.contains_key("Levels") => Some(Value::Bool(false)),
                _ => None,
            },
            Self::NameField if answer.as_array().is_some_and(Vec::is_empty) => {
                Some(json!("no name"))
            }
            Self::NameField => answer.get("name").cloned(),
            Self::SkippedRootKey => answer
                .get("statement")
                .map(|statement| Value::Bool(statement.get("Nested").is_some())),
            Self::FileNamed => answer.get("statement").map(|_| Value::Bool(false)),
            Self::Priority => answer.as_str().map(|rule| Value::String(rule.into())),
            Self::PriorityListed => (!answer.is_null()).then_some(Value::Bool(true)),
            // A block that also takes a bare value (a weight block) compares as both forms.
            Self::ValueForm => {
                let config_form = |answer: &Value| {
                    answer.get("form")?.as_str().map(|form| {
                        Value::String(
                            match form {
                                "boolean" => "bool",
                                "integer" => "int",
                                "number" => "float",
                                other => other,
                            }
                            .into(),
                        )
                    })
                };
                let form = config_form(answer)?;

                match answer.get("scalar").and_then(config_form) {
                    Some(scalar) => Some(json!([form, scalar])),
                    None => Some(form),
                }
            }
            Self::Cardinality => match answer.as_str() {
                // No repeat behavior rejects an occurrence, so none states a finite maximum.
                Some("Replace" | "Accumulate" | "Merges") => Some(Value::String("inf".into())),
                _ => answer.as_u64().map(Value::from),
            },
            Self::EnumMember => answer.as_bool().map(Value::Bool),
            Self::PushScope => {
                let scopes: BTreeSet<_> = answer
                    .as_array()?
                    .iter()
                    .map(Value::as_str)
                    .collect::<Option<_>>()?;

                (!scopes.contains("enclosing")).then(|| json!(scopes))
            }
        }
    }

    fn agrees(&self, config: &Value, atlas: &Value) -> bool {
        if config == atlas {
            return true;
        }

        if let (Self::ValueForm, Some(forms)) = (self, atlas.as_array()) {
            return forms.iter().any(|form| self.agrees(config, form));
        }

        if matches!(self, Self::PushScope) {
            return config == "any";
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
    scopes: &BTreeMap<String, BTreeSet<String>>,
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
    let Some(kind) = ComparisonKind::for_claim(claim) else {
        return (
            Status::MissingFromAtlas,
            Some("Config assertion has no comparison mapping for this property".into()),
        );
    };
    let Some(config) = kind.config_value(&claim.config_answer, scopes) else {
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
    scopes: &BTreeMap<String, BTreeSet<String>>,
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
    let (status, reason) = comparison_outcome(claim, answers, &gaps, scopes);
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

/// The loaded answer must be the one that the snapshot summarizes: same build, same content,
/// same counts and same gaps. A recording of other content would otherwise mix silently into the
/// comparison.
fn check_loaded_modifiers(
    rules: &snapshot::Snapshot,
    loaded: &pdx_native::Answer<LoadedModifiers>,
) -> Result<(), String> {
    if rules.applicability.builds != [loaded.source.build.clone()] {
        return Err("The loaded modifier answer is for another build than the snapshot".into());
    }

    let recorded = rules
        .rules
        .iter()
        .find(|rule| rule.id == "inventory:loaded_modifiers#loaded_summary")
        .ok_or("The snapshot has no loaded modifier summary to check the answer against")?;
    let matches = snapshot::loaded_summary(&loaded.value).is_some_and(|(condition, summary)| {
        recorded.conditions == [condition] && recorded.answer == summary
    }) && rules
        .answers
        .get("loaded_modifiers")
        .is_some_and(|answer| answer.native_gaps == loaded.gaps);

    if !matches {
        return Err(
            "The loaded modifier answer is not the one that the snapshot summarizes".into(),
        );
    }

    Ok(())
}

/// Compares an Atlas rule snapshot directly with the config ledger and the supplied inputs.
/// Recorded answers remain comparable, but this report grants no coverage credit.
pub fn evaluate(ledger: &Ledger, input: &[u8], inputs: &Inputs) -> Result<Report, String> {
    let rules: snapshot::Snapshot =
        serde_json::from_slice(input).map_err(|error| format!("Invalid rule snapshot: {error}"))?;
    if let Some(loaded) = inputs.loaded_modifiers {
        check_loaded_modifiers(&rules, loaded)?;
    }
    let types = super::ConfigTypes::new(ledger);
    let projection = projection::project_for_comparison(ledger, &types, &rules)?;
    let answers_by_question = question_index(&projection.answers, |answer: &Answer| {
        (&answer.question, &answer.conditions)
    });
    let gaps_by_question = question_index(&projection.gaps, |gap: &Gap| {
        (&gap.question, &gap.conditions)
    });
    let scopes = super::language_subject::scope_aliases(ledger, &rules);
    let mut entries = ledger
        .claims
        .iter()
        .map(|claim| entry(claim, &answers_by_question, &gaps_by_question, &scopes))
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
        format_version: 3,
        config_sha256: ledger.config_sha256.clone(),
        snapshot_sha256: format!("{:x}", Sha256::digest(input)),
        snapshot_id: projection.snapshot_id,
        inventory_complete: ledger.diagnostics.is_empty(),
        entries,
        name_lists: name_lists::config_lists(ledger, &types, &engine, inputs),
        script_docs: script_docs::compare_logs(&engine, inputs)?,
        define_files: name_lists::define_files(&engine, inputs)?,
        loaded_modifiers_read: inputs.loaded_modifiers.is_some(),
        modifier_tags: name_lists::modifier_tags(&engine, inputs),
        entry_scope_checks: crate::callback_checks::for_snapshot(&rules),
    })
}
