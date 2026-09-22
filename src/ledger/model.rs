use pdxscript::Span;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Authority responsible for supplying a question's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// Rules or declarations established from engine evidence.
    EngineFact,
    /// Observations or text established from shipped content.
    ContentDerived,
    /// Authoring-tool modeling and advice.
    ConsumerPolicy,
    /// Text owned by its author rather than a game-evidence producer.
    AuthoredText,
}
impl Owner {
    /// Whether the question belongs in the headline Atlas coverage denominator.
    pub fn atlas_owned(self) -> bool {
        matches!(self, Self::EngineFact | Self::ContentDerived)
    }
}
/// Planned extraction route; assignment alone establishes no support.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Method {
    /// Expected method family.
    pub name: String,
}
/// Source assertion and the independent question Atlas needs to answer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claim {
    /// Stable occurrence identifier, independent of answer text and line numbers.
    pub id: String,
    /// Canonical subject/property identity shared by alternative config answers.
    pub question: String,
    /// Relative CWT filename using forward slashes.
    pub file: String,
    /// Structural path identifying the subject, without its answer.
    pub subject: Vec<String>,
    /// Independently assessable property.
    pub property: String,
    /// Structural conditions under which this question is asked.
    pub conditions: Vec<String>,
    /// CWT's assertion, retained for inspection, never used as the scoring oracle.
    pub config_answer: String,
    /// CWT source location.
    pub span: Span,
    /// Verbatim source bytes for this claim's originating syntax.
    pub source_text: String,
    /// Assigned owner of the question.
    pub owner: Owner,
    /// Explanation of the assignment.
    pub ownership_reason: String,
    /// True for unmeasured documentation or an unconfirmed rewritten-source candidate.
    pub provisional_owner: bool,
    /// Required for engine-fact questions; expected, not demonstrated.
    pub expected_method: Option<Method>,
}
/// A diagnostic that prevents claiming a complete inventory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Deterministic identity within this report.
    pub id: String,
    /// Relative filename.
    pub file: String,
    /// Stable diagnostic category.
    pub kind: String,
    /// Source location.
    pub span: Span,
    /// Explanation, including unknown syntax instead of silently dropping it.
    pub message: String,
}
/// Accounting for a meaningful physical source line.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    /// One-based physical line.
    pub number: usize,
    /// Claims sourced on this line, or linked by its structural delimiters.
    pub claims: Vec<String>,
    /// Diagnostics accounting for unparsed source on this line.
    pub diagnostics: Vec<String>,
}
/// Exact input identity and source accounting for one file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    /// Relative filename.
    pub path: String,
    /// SHA-256 of the original bytes.
    pub sha256: String,
    /// Number of physical lines, including a final unterminated line.
    pub physical_lines: usize,
    /// Meaningful lines only; plain comments and blank lines are excluded.
    pub lines: Vec<Line>,
}
/// Deterministic inventory, independent of any supplied snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ledger {
    /// Version of this tool's JSON contract.
    pub format_version: u32,
    /// SHA-256 over sorted relative filenames and their content hashes.
    pub config_sha256: String,
    /// Sorted input files and line accounting.
    pub files: Vec<File>,
    /// Individually assessable source claims, ordered by stable identity.
    pub claims: Vec<Claim>,
    /// All unresolved parsing/classification/accounting problems.
    pub diagnostics: Vec<Diagnostic>,
}
/// Filesystem-independent source input, keyed by relative filename.
pub type Sources = BTreeMap<String, String>;
