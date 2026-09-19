use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Source of documentation text; does not establish the documented game behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Text in an engine documentation dump.
    EngineText,
    /// A comment in the supplied game content.
    ShippedComment,
    /// No matching source found in the supplied corpus.
    Authored,
}
/// Strength of the text comparison, independent of rule qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Match {
    /// Equal after comment-marker removal and whitespace folding only.
    Exact,
    /// Similar text under the documented token comparison; requires review.
    Rewritten,
    /// No matching source in this bounded measurement.
    None,
}
/// A reproducible text location within a hashed input file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    /// Engine dump or shipped comment.
    pub origin: Origin,
    /// Path relative to its explicit input root.
    pub file: String,
    /// SHA-256 of the original file bytes.
    pub sha256: String,
    /// First physical source line, one-based.
    pub line: usize,
    /// Last physical source line, inclusive.
    pub end_line: usize,
    /// Key path to which the comment or engine text is attached.
    pub key: Vec<String>,
    /// Leading, inline, group, block, or engine association.
    pub association: String,
    /// Extracted source text, before comparison normalization.
    pub text: String,
}
/// Documentation attribution. Similarity and source presence never grant rule coverage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribution {
    /// Attributed text source, bounded to the supplied corpus.
    pub origin: Origin,
    /// Exact, rewritten candidate, or absent.
    pub comparison: Match,
    /// All equally best matching locations, in deterministic order.
    pub sources: Vec<Source>,
}
/// Totals for all documentation entries in a family or file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Counts {
    /// Documentation claim occurrences, with adjacent lines grouped as in the ledger.
    pub entries: usize,
    /// Physical CWT documentation lines, including empty documentation lines.
    pub lines: usize,
    /// Entries with an exact source match.
    pub exact: usize,
    /// Entries with only a near-text candidate.
    pub rewritten: usize,
    /// Entries without a source match; never evidence.
    pub authored: usize,
}
/// Complete documentation measurement, including the source-free remainder.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    /// Version of the documentation attribution contract.
    pub format_version: u32,
    /// Config manifest identity copied from the ledger.
    pub config_sha256: String,
    /// Every examined source file, qualified by root kind, and its byte hash.
    pub inputs: BTreeMap<String, String>,
    /// All documentation claims.
    pub totals: Counts,
    /// Complete family measurements, including all type-schema entries.
    pub by_family: BTreeMap<String, Counts>,
    /// Complete per-file measurements.
    pub by_file: BTreeMap<String, Counts>,
    /// Stable ledger claim IDs with no source match.
    pub authored: Vec<String>,
    /// Stable ledger claim IDs requiring rewritten-text review.
    pub rewritten: Vec<String>,
    /// False when source parsing has limitations; unmatched does not prove historical authorship.
    pub source_parse_complete: bool,
    /// Parser limits and source problems; no silent omissions.
    pub diagnostics: Vec<String>,
}
