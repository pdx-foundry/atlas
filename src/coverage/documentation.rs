//! Content-derived documentation answered by the shipped comments of one installation (SDK-718).
use super::{
    Answer, Evidence, Gap, Origin, Status,
    projection::config_directory,
    shipped_comments::{self, Comment},
};
use crate::ledger::{Claim, Ledger, Owner};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Method identity of the evidence that a shipped comment gives.
const METHOD: &str = "shipped-comments/v1";

/// The base-game shipped files of one identified installation build, with their comments parsed.
pub struct Content {
    build: String,
    sha256: String,
    files: usize,
    comments: BTreeMap<String, Vec<Comment>>,
    /// Parse problems of the shipped files. A file that the parser repairs keeps the comments of
    /// the repaired tree; a file that fails to parse keeps only its guide headings (`key: prose`).
    pub diagnostics: Vec<String>,
}

/// What the shipped comments of one installation answered, in the coverage report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentReport {
    /// Native build ID of the installation.
    pub build: String,
    /// SHA-256 over the sorted relative paths and contents of the shipped files.
    pub sha256: String,
    /// Number of shipped files read.
    pub files: usize,
    /// Parse problems of the shipped files.
    pub diagnostics: Vec<String>,
    /// Content-derived documentation questions that every occurrence answers exactly.
    pub answered: usize,
    /// Content-derived documentation questions without an exact match, with the reason.
    pub gaps: Vec<Gap>,
}

impl Content {
    /// Parses the comments of `files`, keyed by their path relative to the installation root
    /// (`common/defines/00_defines.txt`). `build` is the installation's Native build ID; its
    /// comments credit only a rule snapshot of that build.
    pub fn new(build: impl Into<String>, files: &BTreeMap<String, String>) -> Self {
        let mut content = Self {
            build: build.into(),
            sha256: files_sha256(files),
            files: files.len(),
            comments: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        for (file, text) in files {
            let (comments, diagnostics) = shipped_comments::parse(file, text);

            content.diagnostics.extend(diagnostics);

            for comment in comments {
                let key = comment
                    .key
                    .last()
                    .expect("a comment documents a key")
                    .clone();

                content.comments.entry(key).or_default().push(comment);
            }
        }

        content
    }

    /// The installation's Native build ID.
    pub fn build(&self) -> &str {
        &self.build
    }
}

/// A digest of each relative path and its content hash, in path order: equal for the same files
/// wherever the installation is.
fn files_sha256(files: &BTreeMap<String, String>) -> String {
    let mut digest = Sha256::new();

    for (file, text) in files {
        digest.update(format!("{file}\0{:x}\n", Sha256::digest(text.as_bytes())));
    }

    format!("{:x}", digest.finalize())
}

/// How a documentation claim's text compares with the shipped comments in its content directory.
enum Comparison {
    /// Equal after removing comment markers and folding whitespace, at every listed location.
    Exact(Vec<Comment>),
    /// Similar but not equal text; the best candidates.
    Rewritten(Vec<Comment>),
    /// No comment in the claim's directory documents its key with similar text.
    Absent,
}

/// The answers that `content` gives the content-derived documentation questions, and its report
/// with the gaps of the others. A question is answered only when each of its occurrences matches
/// a shipped comment exactly; the answer lists their distinct texts. `qualified` is false when
/// the rule snapshot is a recording, which earns no credit.
pub(super) fn project(
    ledger: &Ledger,
    content: &Content,
    qualified: bool,
) -> (Vec<Answer>, ContentReport) {
    let directories = content_directories(ledger);
    let item_names = item_names(ledger);
    let mut questions: BTreeMap<(&str, &[String]), Vec<Comparison>> = BTreeMap::new();

    for claim in ledger
        .claims
        .iter()
        .filter(|c| c.property == "documentation" && c.owner == Owner::ContentDerived)
    {
        let key = subject_key(claim, &item_names);
        let comparison = compare(
            claim,
            &key,
            &claim_directories(claim, &directories),
            content,
        );

        questions
            .entry((&claim.question, &claim.conditions))
            .or_default()
            .push(comparison);
    }

    let mut answers = Vec::new();
    let mut gaps = Vec::new();

    for ((question, conditions), comparisons) in questions {
        match answer(&comparisons) {
            Ok(comments) => answers.push(Answer {
                question: question.into(),
                conditions: conditions.to_vec(),
                value: comments
                    .iter()
                    .map(|comment| folded(&comment.text))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .map(Value::String)
                    .collect(),
                status: Status::Supported,
                evidence: comments
                    .iter()
                    .map(|comment| Evidence {
                        id: format!(
                            "shipped-comment:{}:{}-{}",
                            comment.file, comment.line, comment.end_line
                        ),
                        method: METHOD.into(),
                        target: content.build.clone(),
                        qualified,
                        origin: Origin::Content,
                    })
                    .collect(),
            }),
            Err(reason) => gaps.push(Gap {
                question: question.into(),
                conditions: conditions.to_vec(),
                reason,
            }),
        }
    }

    let report = ContentReport {
        build: content.build.clone(),
        sha256: content.sha256.clone(),
        files: content.files,
        diagnostics: content.diagnostics.clone(),
        answered: answers.len(),
        gaps,
    };

    (answers, report)
}

/// The exact locations of every occurrence, or the reason of the weakest occurrence.
fn answer(comparisons: &[Comparison]) -> Result<Vec<&Comment>, String> {
    if comparisons.iter().any(|c| matches!(c, Comparison::Absent)) {
        return Err("No shipped comment in the installation has this text".into());
    }

    if let Some(Comparison::Rewritten(candidates)) = comparisons
        .iter()
        .find(|c| matches!(c, Comparison::Rewritten(_)))
    {
        let candidate = &candidates[0];

        return Err(format!(
            "The shipped comment at {}:{}-{} is a rewritten candidate, not the same text",
            candidate.file, candidate.line, candidate.end_line
        ));
    }

    let mut comments: Vec<_> = comparisons
        .iter()
        .flat_map(|c| match c {
            Comparison::Exact(comments) => comments.as_slice(),
            Comparison::Rewritten(_) | Comparison::Absent => &[],
        })
        .collect();

    comments.sort_by(|a, b| (&a.file, a.line, a.end_line).cmp(&(&b.file, b.line, b.end_line)));
    comments.dedup_by(|a, b| (&a.file, a.line, a.end_line) == (&b.file, b.line, b.end_line));

    Ok(comments)
}

/// The content directories that each config file's types load from (`path = game/common/x`).
fn content_directories(ledger: &Ledger) -> BTreeMap<&str, Vec<String>> {
    let mut result: BTreeMap<&str, Vec<String>> = BTreeMap::new();

    for claim in ledger
        .claims
        .iter()
        .filter(|c| c.property == "loader_path" && c.subject.last().is_some_and(|s| s == "path"))
    {
        let path = config_directory(&claim.config_answer).trim_end_matches('/');

        result.entry(&claim.file).or_default().push(path.into());
    }

    for paths in result.values_mut() {
        paths.sort();
        paths.dedup();
    }

    result
}

/// The directories whose comments can document `claim`: the define, on_action and game rule
/// folders, else the folders that the file's types load from, else the file's own stem.
fn claim_directories(claim: &Claim, directories: &BTreeMap<&str, Vec<String>>) -> Vec<String> {
    let name = claim.file.rsplit('/').next().unwrap_or(&claim.file);

    if claim.file.starts_with("common/defines/") {
        return vec!["common/defines".into()];
    }

    if let Some(stem) = ["on_actions.cwt", "game_rules.cwt"]
        .contains(&name)
        .then(|| name.trim_end_matches(".cwt"))
    {
        return vec![format!("common/{stem}")];
    }

    directories
        .get(claim.file.as_str())
        .cloned()
        .unwrap_or_else(|| vec![claim.file.trim_end_matches(".cwt").into()])
}

/// The value of each bare item, by (file, subject): an `on_actions.cwt` item is documented under
/// its name, not its position.
fn item_names(ledger: &Ledger) -> BTreeMap<(&str, &[String]), &str> {
    ledger
        .claims
        .iter()
        .filter(|c| c.property == "value_form")
        .map(|c| {
            (
                (c.file.as_str(), c.subject.as_slice()),
                c.config_answer.as_str(),
            )
        })
        .collect()
}

/// The key that a shipped comment must document to describe `claim`.
fn subject_key(claim: &Claim, item_names: &BTreeMap<(&str, &[String]), &str>) -> String {
    let last = claim.subject.last().map(String::as_str).unwrap_or("");

    if last.starts_with("$item:") {
        return item_names
            .get(&(claim.file.as_str(), claim.subject.as_slice()))
            .map_or_else(String::new, |name| (*name).into());
    }

    if let Some(alias) = last
        .strip_prefix("alias[")
        .and_then(|s| s.strip_suffix(']'))
    {
        return alias.split_once(':').map_or(alias, |(_, key)| key).into();
    }

    last.into()
}

/// Compares `claim`'s text with each comment that documents `key` in one of `directories`
/// (descendant folders included). A define comment must also sit under the claim's namespace.
fn compare(claim: &Claim, key: &str, directories: &[String], content: &Content) -> Comparison {
    let text = folded(&claim.config_answer);
    let mut best_score = 0;
    let mut best = Vec::new();

    for comment in content.comments.get(key).into_iter().flatten() {
        let folder = comment
            .file
            .rsplit_once('/')
            .map_or("", |(folder, _)| folder);
        let in_directory = directories.iter().any(|directory| {
            folder == directory
                || folder
                    .strip_prefix(directory.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        });
        let in_namespace = !claim.file.starts_with("common/defines/")
            || (comment.key.len() + 1 == claim.subject.len()
                && claim.subject.ends_with(&comment.key));

        if !in_directory || !in_namespace {
            continue;
        }

        let (score, excerpts) = best_excerpts(&text, comment);

        if score > best_score {
            best_score = score;
            best.clear();
        }

        if score == best_score {
            best.extend(excerpts);
        }
    }

    match best_score {
        0 => Comparison::Absent,
        EXACT => Comparison::Exact(best),
        _ => Comparison::Rewritten(best),
    }
}

/// The similarity score of equal text.
const EXACT: usize = 10_000;

fn folded(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `EXACT` for equal nonempty text; otherwise the word edit similarity in ten-thousandths, or 0
/// when either side has fewer than 8 words or the score is below 85%. Case is ignored for words.
fn similarity(left: &str, right: &str) -> usize {
    if left == right && !left.is_empty() {
        return EXACT;
    }

    let a: Vec<_> = left.split_whitespace().collect();
    let b: Vec<_> = right.split_whitespace().collect();

    if a.len().min(b.len()) < 8 || a.len().min(b.len()) * 100 < a.len().max(b.len()) * 85 {
        return 0;
    }

    let mut row: Vec<usize> = (0..=b.len()).collect();

    for (i, word) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;

        for (j, other) in b.iter().enumerate() {
            let previous = row[j + 1];
            row[j + 1] = (diagonal + usize::from(!word.eq_ignore_ascii_case(other)))
                .min(row[j] + 1)
                .min(previous + 1);
            diagonal = previous;
        }
    }

    let score = (a.len().max(b.len()) - row[b.len()]) * EXACT / a.len().max(b.len());

    if score >= 8_500 {
        score.min(EXACT - 1)
    } else {
        0
    }
}

/// The best score of `text` against each consecutive line range of `comment`, and every range
/// with that score. A range lets a description match without the usage example that follows it.
fn best_excerpts(text: &str, comment: &Comment) -> (usize, Vec<Comment>) {
    let lines: Vec<_> = comment.text.lines().collect();
    let mut best = Vec::new();
    let mut score = 0;

    for first in 0..lines.len() {
        if lines[first].trim().is_empty() {
            continue;
        }

        let mut excerpt = String::new();

        for (last, line) in lines.iter().enumerate().skip(first) {
            if last > first {
                excerpt.push('\n');
            }

            excerpt.push_str(line);

            let normalized = folded(&excerpt);
            let candidate = similarity(text, &normalized);

            if candidate > score {
                score = candidate;
                best.clear();
            }

            if candidate == score && score > 0 && !line.trim().is_empty() {
                best.push(Comment {
                    line: comment.line + first,
                    end_line: comment.line + last,
                    text: excerpt.clone(),
                    ..comment.clone()
                });
            }

            if normalized.len() > text.len() * 2 + 100 {
                break;
            }
        }
    }

    (score, best)
}
