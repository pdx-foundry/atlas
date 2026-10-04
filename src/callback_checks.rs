//! Exact-build callback checks retained from SDK-608. Expectations never feed Native.

use crate::snapshot::{Snapshot, SubjectKind};
use pdx_native::{BuildId, Source};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Historical independent checks of callback entry scopes, not a fresh config comparison.
#[derive(Debug, Serialize, Deserialize)]
pub struct EntryScopeChecks {
    /// Exact engine build checked by hand.
    pub build: BuildId,
    /// Native callback method checked by hand.
    pub method: String,
    /// Origin of the recorded conclusions.
    pub provenance: String,
    /// Agreement counts for the full historical comparison: `on_actions` against vanilla
    /// scope comments, and `game_rules` against config `replace_scopes`.
    pub agreements: std::collections::BTreeMap<String, AgreementCount>,
    /// Confirmed disagreements and absent independent evidence. These groups do not fully
    /// account for `total - agree`; other Native gaps remain in the snapshot.
    pub groups: Vec<EntryScopeCheck>,
}

/// Historical number of names agreeing with their independent expectation.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgreementCount {
    /// Names with agreeing entry scopes.
    pub agree: usize,
    /// Names included in the comparison, including gaps.
    pub total: usize,
}

/// A shared check result for named callbacks on one exact build.
#[derive(Debug, Serialize, Deserialize)]
pub struct EntryScopeCheck {
    /// Whether these names are on_actions or game rules.
    pub kind: SubjectKind,
    /// Result of the independent check.
    pub outcome: CheckOutcome,
    /// Independent expectation source.
    pub source: String,
    /// Stable explanation of the finding.
    pub reason: String,
    /// Callback names to which the finding applies.
    pub names: Vec<String>,
}

/// Whether a hand check confirmed an engine answer or lacks independent evidence.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    /// The engine confirms Native despite a config disagreement.
    ConfigDisagreementEngineConfirmed,
    /// The engine confirms Native despite a vanilla-comment disagreement.
    CommentDisagreementEngineConfirmed,
    /// There is no independent source for the entry scopes.
    NoIndependentSource,
}

static CHECKS: LazyLock<EntryScopeChecks> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../docs/coverage/entry-scope-checks.json"))
        .expect("checked-in entry-scope checks must be valid")
});

fn for_source(source: &Source) -> Option<&'static EntryScopeChecks> {
    (source.build == CHECKS.build && source.method == CHECKS.method).then_some(&CHECKS)
}

pub(crate) fn for_snapshot(snapshot: &Snapshot) -> Option<&'static EntryScopeChecks> {
    for key in ["on_actions", "game_rules"] {
        let answer = snapshot.answers.get(key)?;
        let source = snapshot.sources.get(&answer.source)?;
        for_source(source)?;
    }

    Some(&CHECKS)
}

pub(crate) fn missing_source(
    source: &Source,
    kind: SubjectKind,
    name: &str,
) -> Option<&'static str> {
    for_source(source)?
        .groups
        .iter()
        .find(|check| {
            check.kind == kind
                && check.outcome == CheckOutcome::NoIndependentSource
                && check.names.iter().any(|candidate| candidate == name)
        })
        .map(|check| check.reason.as_str())
}
