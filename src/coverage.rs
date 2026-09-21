//! Coverage of questions, independent of whether Atlas and CWT give the same answer.
/// Direct comparison of Atlas rule answers with config assertions.
pub mod comparison;
mod rules;
use crate::ledger::{Ledger, Owner};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Qualification state of an exact answer, not a correctness grade for CWT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Qualified answer under the recorded conditions.
    Supported,
    /// Only part of the question was established.
    Partial,
    /// Not examined.
    Untested,
    /// Answer not established.
    Unknown,
    /// Outside this snapshot's declared support.
    Unsupported,
    /// Applicable evidence remains in unresolved conflict.
    Conflicted,
}
/// Origin of supporting material, preventing synthetic controls from becoming production support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Captured/qualified engine evidence.
    Engine,
    /// Observed shipped content.
    Content,
    /// Author or consumer-owned material.
    Authored,
    /// Test-only fixture.
    Synthetic,
}
/// Traceable qualification for a precise snapshot answer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Stable evidence identity or durable locator.
    pub id: String,
    /// Extraction or validation method identity.
    pub method: String,
    /// Exact target identity for this evidence.
    pub target: String,
    /// Whether the method/evidence was qualified for this property.
    pub qualified: bool,
    /// Origin of the evidence.
    pub origin: Origin,
}
/// One answer to a ledger question, under explicit conditions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// Ledger question identity (not occurrence identity or config answer).
    pub question: String,
    /// Exact context conditions; narrower answers do not establish a broader question.
    pub conditions: Vec<String>,
    /// Atlas's answer; equality with the CWT answer has no role in coverage.
    pub value: Value,
    /// Qualification/knowledge state.
    pub status: Status,
    /// Evidence supporting this answer.
    pub evidence: Vec<Evidence>,
}
/// Explicit unresolved question supplied by the snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    /// Ledger question identity.
    pub question: String,
    /// Conditions to which this gap applies.
    pub conditions: Vec<String>,
    /// What remains unknown.
    pub reason: String,
}
/// Bounded input contract for the scoreboard, not the full Atlas publication envelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Must be `atlas_coverage`.
    pub kind: String,
    /// Supported contract version, currently 1.
    pub format_version: u32,
    /// Immutable snapshot identity.
    pub snapshot_id: String,
    /// Exact target for all credited evidence in this report.
    pub target: String,
    /// Source-linked property answers.
    pub answers: Vec<Answer>,
    /// Known gaps; an applicable gap prevents claiming complete coverage.
    pub gaps: Vec<Gap>,
}
/// Coverage fraction. Empty denominators have no percentage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fraction {
    /// Number of questions answered completely with applicable evidence.
    pub covered: usize,
    /// Number of inventoried questions.
    pub total: usize,
    /// Percentage rounded to six decimal places, or null for an empty denominator.
    pub percent: Option<f64>,
}
impl Fraction {
    fn add(&mut self, covered: bool) {
        self.total += 1;
        self.covered += usize::from(covered);
    }
    fn finish(&mut self) {
        self.percent = (self.total != 0).then(|| {
            ((self.covered as f64 / self.total as f64) * 100_000_000.0).round() / 1_000_000.0
        });
    }
}
/// The two denominator views and breakdown by owner.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Totals {
    /// Headline: engine-fact and content-derived questions only.
    pub atlas_owned: Fraction,
    /// Every inventoried claim, including consumer policy and authored text.
    pub all_claims: Fraction,
    /// Separate fractions for all four authority classes.
    pub by_owner: BTreeMap<Owner, Fraction>,
}
impl Totals {
    fn new() -> Self {
        Self {
            by_owner: [
                Owner::EngineFact,
                Owner::ContentDerived,
                Owner::ConsumerPolicy,
                Owner::AuthoredText,
            ]
            .into_iter()
            .map(|o| (o, Fraction::default()))
            .collect(),
            ..Self::default()
        }
    }
    fn add(&mut self, owner: Owner, covered: bool) {
        self.all_claims.add(covered);
        if owner.atlas_owned() {
            self.atlas_owned.add(covered);
        }
        self.by_owner.entry(owner).or_default().add(covered);
    }
    fn finish(&mut self) {
        self.all_claims.finish();
        self.atlas_owned.finish();
        for f in self.by_owner.values_mut() {
            f.finish();
        }
    }
}
/// Per-claim support result, without an agreement/correctness score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assessment {
    /// Ledger occurrence identity.
    pub claim: String,
    /// Whether Atlas answers this question completely.
    pub covered: bool,
    /// Why credit was granted or withheld.
    pub reason: String,
    /// Distinct states supplied by applicable answers, including incomplete answers.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub answer_states: Vec<Status>,
    /// Evidence identities used for credited answers.
    pub evidence: Vec<String>,
}
/// Deterministic scoreboard tied to exact config and snapshot bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    /// Report contract version.
    pub format_version: u32,
    /// Source config content identity.
    pub config_sha256: String,
    /// Supplied snapshot byte identity, absent when no snapshot was supplied.
    pub snapshot_sha256: Option<String>,
    /// Input snapshot identity when supplied in the coverage contract.
    pub snapshot_id: Option<String>,
    /// Target to which credited answers apply.
    pub target: Option<String>,
    /// False when unresolved source diagnostics remain; percentages describe only inventoried claims.
    pub inventory_complete: bool,
    /// Overall totals.
    pub totals: Totals,
    /// Per-file totals, including empty files.
    pub by_file: BTreeMap<String, Totals>,
    /// Each source claim's support assessment.
    pub claims: Vec<Assessment>,
    /// Registry observations retained as observations, with no rule credit.
    pub registry_observations: Vec<Value>,
    /// Snapshot questions without a matching config question; excluded from the denominator.
    pub atlas_only_questions: Vec<String>,
}
fn validate(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.kind != "atlas_coverage" || snapshot.format_version != 1 {
        return Err("Unsupported coverage input contract".into());
    }
    if snapshot.snapshot_id.trim().is_empty() || snapshot.target.trim().is_empty() {
        return Err("Snapshot identity and exact target must be nonempty".into());
    }
    for gap in &snapshot.gaps {
        if gap.question.trim().is_empty() || gap.reason.trim().is_empty() {
            return Err("Gaps require a question and a nonempty explanation".into());
        }
    }
    for answer in &snapshot.answers {
        if answer.question.trim().is_empty() || answer.value.is_null() {
            return Err("Answers require a question and a non-null answer".into());
        }
        if answer.evidence.iter().any(|e| {
            e.id.trim().is_empty() || e.method.trim().is_empty() || e.target.trim().is_empty()
        }) {
            return Err("Evidence requires an identity, method and target".into());
        }
    }
    Ok(())
}
fn suitable(owner: Owner, e: &Evidence, target: &str) -> bool {
    e.qualified
        && e.target == target
        && match owner {
            Owner::EngineFact => e.origin == Origin::Engine,
            Owner::ContentDerived => e.origin == Origin::Content,
            Owner::ConsumerPolicy | Owner::AuthoredText => e.origin == Origin::Authored,
        }
}
struct SnapshotIndex<'a> {
    target: &'a str,
    answers: BTreeMap<&'a str, Vec<&'a Answer>>,
    gaps: BTreeMap<&'a str, Vec<&'a Gap>>,
}
impl<'a> SnapshotIndex<'a> {
    fn new(snapshot: &'a Snapshot) -> Self {
        let mut index = Self {
            target: &snapshot.target,
            answers: BTreeMap::new(),
            gaps: BTreeMap::new(),
        };
        for answer in &snapshot.answers {
            index
                .answers
                .entry(&answer.question)
                .or_default()
                .push(answer);
        }
        for gap in &snapshot.gaps {
            index.gaps.entry(&gap.question).or_default().push(gap);
        }
        index
    }
}

fn assessment(claim: &crate::ledger::Claim, snapshot: Option<&SnapshotIndex<'_>>) -> Assessment {
    let mut result = Assessment {
        claim: claim.id.clone(),
        covered: false,
        reason: "No qualified answer to this question".into(),
        evidence: Vec::new(),
        answer_states: Vec::new(),
    };
    let Some(snapshot) = snapshot else {
        return result;
    };
    let answers: Vec<_> = snapshot
        .answers
        .get(claim.question.as_str())
        .into_iter()
        .flatten()
        .filter(|a| a.conditions == claim.conditions)
        .collect();
    result.answer_states = answers
        .iter()
        .map(|a| a.status)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !answers.is_empty() {
        result.reason = "Applicable answers do not establish complete qualified support".into();
    }
    if snapshot
        .gaps
        .get(claim.question.as_str())
        .into_iter()
        .flatten()
        .any(|g| g.conditions == claim.conditions)
    {
        result.reason = "Snapshot records an applicable gap".into();
        return result;
    }
    if answers.iter().any(|a| a.status == Status::Conflicted) {
        result.reason = "Unresolved evidence conflict".into();
        return result;
    }
    let qualified: Vec<_> = answers
        .into_iter()
        .filter(|a| {
            a.status == Status::Supported
                && a.evidence
                    .iter()
                    .any(|e| suitable(claim.owner, e, snapshot.target))
        })
        .collect();
    if qualified.is_empty() {
        return result;
    }
    if qualified.iter().any(|a| a.value != qualified[0].value) {
        result.reason = "Conflicting qualified snapshot answers".into();
        return result;
    }
    result.covered = true;
    result.reason =
        "Atlas has an applicable, evidence-backed answer; CWT agreement is not required".into();
    result.evidence = qualified
        .iter()
        .flat_map(|a| a.evidence.iter())
        .filter(|e| suitable(claim.owner, e, snapshot.target))
        .map(|e| e.id.clone())
        .collect();
    result.evidence.sort();
    result.evidence.dedup();
    result
}
fn registry_observations(value: &Value) -> Result<Vec<Value>, String> {
    let live = value.get("queries").and_then(Value::as_object);
    let replay = value
        .get("startup")
        .and_then(Value::as_object)
        .filter(|_| value.get("final_snapshots").is_some());
    let queries = live
        .or(replay)
        .ok_or("Input is neither an Atlas coverage snapshot nor a registry caller result")?;
    let mut observations = Vec::new();
    for (name, answer) in queries {
        if let Some(ok) = answer.get("Ok") {
            let native = ok
                .get("native")
                .and_then(Value::as_object)
                .ok_or("Invalid registry observation: native result missing")?;
            let items = native
                .get("registeredItems")
                .and_then(Value::as_array)
                .ok_or("Invalid registry observation: items missing")?;
            observations.push(serde_json::json!({"registry":name,"observed_items":items.len(),"items":items,"activation":native.get("activation"),"completion":native.get("completion"),"origin":native.get("origin"),"limits":native.get("limits"),"gaps":ok.get("gaps"),"rule_credit":false}));
        } else if answer.get("Err").is_some() {
            observations
                .push(serde_json::json!({"registry":name,"unavailable":true,"rule_credit":false}));
        } else {
            return Err("Invalid registry result variant".into());
        }
    }
    Ok(observations)
}
/// Scores exact questions under the supplied evidence contract. Config answer equality is never tested.
/// Invalid snapshot formats return an error rather than a successful zero-coverage report.
pub fn evaluate(ledger: &Ledger, input: Option<&[u8]>) -> Result<Report, String> {
    let snapshot_sha256 = input.map(|b| format!("{:x}", Sha256::digest(b)));
    let mut observations = Vec::new();
    let snapshot = if let Some(bytes) = input {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|e| format!("Invalid snapshot JSON: {e}"))?;
        if value.get("kind").and_then(Value::as_str) == Some("atlas_coverage") {
            let snapshot: Snapshot = serde_json::from_value(value)
                .map_err(|e| format!("Invalid coverage input: {e}"))?;
            validate(&snapshot)?;
            Some(snapshot)
        } else if value.get("kind").and_then(Value::as_str) == Some("atlas_rule_snapshot") {
            let rule_snapshot: crate::snapshot::Snapshot = serde_json::from_value(value)
                .map_err(|e| format!("Invalid rule snapshot input: {e}"))?;
            let snapshot = rules::project(ledger, &rule_snapshot)?;
            validate(&snapshot)?;
            Some(snapshot)
        } else {
            observations = registry_observations(&value)?;
            None
        }
    } else {
        None
    };
    let mut totals = Totals::new();
    let mut by_file: BTreeMap<_, _> = ledger
        .files
        .iter()
        .map(|f| (f.path.clone(), Totals::new()))
        .collect();
    let mut claims = Vec::new();
    let index = snapshot.as_ref().map(SnapshotIndex::new);
    for claim in &ledger.claims {
        let answer = assessment(claim, index.as_ref());
        totals.add(claim.owner, answer.covered);
        by_file
            .get_mut(&claim.file)
            .expect("claim file")
            .add(claim.owner, answer.covered);
        claims.push(answer);
    }
    totals.finish();
    for totals in by_file.values_mut() {
        totals.finish();
    }
    let questions: BTreeSet<_> = ledger.claims.iter().map(|c| c.question.as_str()).collect();
    let atlas_only_questions = snapshot
        .as_ref()
        .map(|s| {
            s.answers
                .iter()
                .map(|a| a.question.as_str())
                .chain(s.gaps.iter().map(|g| g.question.as_str()))
                .filter(|question| !questions.contains(question))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();
    Ok(Report {
        format_version: 1,
        config_sha256: ledger.config_sha256.clone(),
        snapshot_sha256,
        snapshot_id: snapshot.as_ref().map(|s| s.snapshot_id.clone()),
        target: snapshot.as_ref().map(|s| s.target.clone()),
        inventory_complete: ledger.diagnostics.is_empty(),
        totals,
        by_file,
        claims,
        registry_observations: observations,
        atlas_only_questions,
    })
}
