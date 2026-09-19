//! Documentation text attribution against explicit engine dumps and installed content.
//! This comparison supplies provenance, never qualified rule answers.
mod comments;
mod engine;
mod model;
use crate::ledger::{Claim, Ledger, Method, Owner};
pub use comments::parse_comments;
pub use engine::parse_engine_docs;
pub use model::*;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, path::Path};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Hashed, parsed text sources for one bounded documentation measurement.
pub struct Corpus {
    sources: BTreeMap<String, Vec<Source>>,
    inputs: BTreeMap<String, String>,
    /// Source parsing problems retained in the report; callers must surface these limits.
    pub diagnostics: Vec<String>,
}
impl Corpus {
    /// Builds a corpus from relative game-content paths and effects/triggers log filenames.
    /// All files, including files with parsing problems, remain in the input manifest.
    pub fn from_sources(
        content: &BTreeMap<String, String>,
        engine: &BTreeMap<String, String>,
    ) -> Self {
        let mut corpus = Self {
            sources: BTreeMap::new(),
            inputs: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        for (file, text) in content {
            corpus
                .inputs
                .insert(format!("content/{file}"), hash(text.as_bytes()));
            let (sources, diagnostics) = parse_comments(file, text);
            corpus.diagnostics.extend(diagnostics);
            for source in sources {
                corpus
                    .sources
                    .entry(source.key.last().unwrap().clone())
                    .or_default()
                    .push(source);
            }
        }
        for (file, text) in engine {
            corpus
                .inputs
                .insert(format!("engine/{file}"), hash(text.as_bytes()));
            let sources = parse_engine_docs(file, text);
            if sources.is_empty() {
                corpus
                    .diagnostics
                    .push(format!("{file}: no engine declarations parsed"));
            }
            for source in sources {
                corpus
                    .sources
                    .entry(source.key[0].clone())
                    .or_default()
                    .push(source);
            }
        }
        corpus
    }
}
fn read_content(root: &Path, dir: &Path, output: &mut BTreeMap<String, String>) -> io::Result<()> {
    if fs::symlink_metadata(dir)?.is_symlink() {
        return Err(io::Error::other(format!(
            "Symlink source is not supported: {}",
            dir.display()
        )));
    }
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_symlink() {
            return Err(io::Error::other(format!(
                "Symlink source is not supported: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            read_content(root, &path, output)?;
        } else if path.extension().is_some_and(|ext| ext == "txt") {
            let file = path
                .strip_prefix(root)
                .expect("descendant")
                .to_str()
                .ok_or_else(|| io::Error::other("Non-UTF-8 content filename"))?
                .replace('\\', "/");
            output.insert(
                file,
                fs::read_to_string(&path).map_err(|error| {
                    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
                })?,
            );
        }
    }
    Ok(())
}
/// Reads base-game `common/**/*.txt` and the supplied dump's effects.log/triggers.log.
/// Rejects missing, empty, unreadable, non-UTF-8, or symlinked inputs.
pub fn read_corpus(game: &Path, engine: &Path) -> io::Result<Corpus> {
    for root in [game, engine] {
        if fs::symlink_metadata(root)?.is_symlink() {
            return Err(io::Error::other("Source root must not be a symlink"));
        }
    }
    let mut content = BTreeMap::new();
    read_content(game, &game.join("common"), &mut content)?;
    if content.is_empty() {
        return Err(io::Error::other("No common content files found"));
    }
    let mut dumps: BTreeMap<String, String> = BTreeMap::new();
    for file in ["effects.log", "triggers.log"] {
        let path = engine.join(file);
        if fs::symlink_metadata(&path)?.is_symlink() {
            return Err(io::Error::other("Engine dump must not be a symlink"));
        }
        dumps.insert(file.into(), fs::read_to_string(path)?);
    }
    for (file, text) in &dumps {
        if parse_engine_docs(file, text).is_empty() {
            return Err(io::Error::other(format!(
                "{file}: no engine declarations parsed"
            )));
        }
    }
    Ok(Corpus::from_sources(&content, &dumps))
}
fn folded(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn similarity(left: &str, right: &str) -> usize {
    if left == right && !left.is_empty() {
        return 10000;
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
    let score = (a.len().max(b.len()) - row[b.len()]) * 10000 / a.len().max(b.len());
    if score >= 8500 { score.min(9999) } else { 0 }
}
fn best_excerpts(doc: &str, source: &Source) -> (usize, Vec<Source>) {
    let lines: Vec<_> = source.text.lines().collect();
    let mut best = Vec::new();
    let mut score = 0;
    for first in 0..lines.len() {
        if lines[first].trim().is_empty() {
            continue;
        }
        let mut text = String::new();
        for (last, line) in lines.iter().enumerate().skip(first) {
            if last > first {
                text.push('\n');
            }
            text.push_str(line);
            let normalized = folded(&text);
            let candidate = similarity(doc, &normalized);
            if candidate > score {
                score = candidate;
                best.clear();
            }
            if candidate == score && score > 0 && !line.trim().is_empty() {
                let mut excerpt = source.clone();
                excerpt.line += first;
                excerpt.end_line = source.line + last;
                excerpt.text = text.clone();
                best.push(excerpt);
            }
            if normalized.len() > doc.len() * 2 + 100 {
                break;
            }
        }
    }
    (score, best)
}
fn family(claim: &Claim) -> &'static str {
    if claim.file == "effects.cwt" {
        "effects"
    } else if claim.file == "triggers.cwt" {
        "triggers"
    } else if claim.file.starts_with("common/defines/") {
        "defines"
    } else if claim.file == "on_actions.cwt" {
        "on_actions"
    } else if claim.file == "game_rules.cwt" {
        "game_rules"
    } else if claim.file.starts_with("common/") {
        "type_schemas"
    } else {
        "other"
    }
}
fn subject_key(claim: &Claim, ledger: &Ledger) -> String {
    if ["effects.cwt", "triggers.cwt"].contains(&claim.file.as_str()) {
        return claim
            .subject
            .iter()
            .find_map(|part| {
                part.strip_prefix("alias[")
                    .and_then(|part| part.strip_suffix(']'))
                    .and_then(|part| part.split_once(':'))
                    .map(|(_, key)| key.to_string())
            })
            .unwrap_or_default();
    }
    let last = claim.subject.last().map(String::as_str).unwrap_or("");
    if last.starts_with("$item:") {
        return ledger
            .claims
            .iter()
            .find(|other| {
                other.file == claim.file
                    && other.subject == claim.subject
                    && other.property == "value_form"
            })
            .map(|other| other.config_answer.clone())
            .unwrap_or_default();
    }
    if let Some(alias) = last
        .strip_prefix("alias[")
        .and_then(|s| s.strip_suffix(']'))
    {
        return alias.split_once(':').map_or(alias, |(_, key)| key).into();
    }
    last.into()
}
fn directories(ledger: &Ledger) -> BTreeMap<String, Vec<String>> {
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for claim in &ledger.claims {
        if claim.property == "loader_path" && claim.subject.last().is_some_and(|s| s == "path") {
            let path = claim
                .config_answer
                .trim_matches('"')
                .strip_prefix("game/")
                .unwrap_or(&claim.config_answer)
                .trim_end_matches('/')
                .to_string();
            result.entry(claim.file.clone()).or_default().push(path);
        }
    }
    for paths in result.values_mut() {
        paths.sort();
        paths.dedup();
    }
    result
}
fn attribute(claim: &Claim, key: &str, paths: &[String], corpus: &Corpus) -> Attribution {
    let doc = folded(&claim.config_answer);
    let mut best_score = 0;
    let mut sources = Vec::new();
    for source in corpus.sources.get(key).into_iter().flatten() {
        let in_directory = match source.origin {
            Origin::EngineText => paths
                .iter()
                .any(|path| source.file == format!("{path}.log")),
            Origin::ShippedComment => {
                let directory = source
                    .file
                    .rsplit_once('/')
                    .map_or("", |(directory, _)| directory);
                paths.iter().any(|path| {
                    directory == path
                        || directory
                            .strip_prefix(path.as_str())
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
            }
            Origin::Authored => false,
        };
        if !in_directory {
            continue;
        }
        if family(claim) == "defines"
            && (source.key.len() + 1 != claim.subject.len()
                || !claim.subject.ends_with(&source.key))
        {
            continue;
        }
        let (score, excerpts) = best_excerpts(&doc, source);
        if score > best_score {
            best_score = score;
            sources.clear();
        }
        if score == best_score {
            sources.extend(excerpts);
        }
    }
    sources.sort_by(|a, b| {
        (&a.file, a.line, a.end_line, &a.key, &a.association).cmp(&(
            &b.file,
            b.line,
            b.end_line,
            &b.key,
            &b.association,
        ))
    });
    sources.dedup_by(|a, b| {
        a.file == b.file && a.line == b.line && a.end_line == b.end_line && a.key == b.key
    });
    Attribution {
        origin: sources.first().map_or(Origin::Authored, |s| s.origin),
        comparison: if best_score == 10000 {
            Match::Exact
        } else if best_score > 0 {
            Match::Rewritten
        } else {
            Match::None
        },
        sources,
    }
}
fn count(counts: &mut Counts, claim: &Claim, attribution: &Attribution) {
    counts.entries += 1;
    counts.lines += claim.span.end_line - claim.span.line + 1;
    match attribution.comparison {
        Match::Exact => counts.exact += 1,
        Match::Rewritten => counts.rewritten += 1,
        Match::None => counts.authored += 1,
    }
}
/// Tags every documentation claim and measures every family. Unmatched entries are authored within
/// this corpus, not proof of historical authorship. Rewritten matches are review candidates only.
/// Claim identities stay fixed; ownership follows the found text source and no evidence is created.
pub fn annotate(ledger: &mut Ledger, corpus: &Corpus) -> Report {
    let paths = directories(ledger);
    let attributions: Vec<_> = ledger
        .claims
        .iter()
        .filter(|c| c.property == "documentation")
        .map(|claim| {
            let key = subject_key(claim, ledger);
            let family = family(claim);
            let locations = match family {
                "effects" | "triggers" => vec![family.into()],
                "defines" | "on_actions" | "game_rules" => vec![format!("common/{family}")],
                _ => paths
                    .get(&claim.file)
                    .cloned()
                    .unwrap_or_else(|| vec![claim.file.trim_end_matches(".cwt").into()]),
            };
            (claim.id.clone(), attribute(claim, &key, &locations, corpus))
        })
        .collect();
    let mut report = Report {
        format_version: 1,
        config_sha256: ledger.config_sha256.clone(),
        inputs: corpus.inputs.clone(),
        totals: Counts::default(),
        by_family: BTreeMap::new(),
        by_file: BTreeMap::new(),
        authored: Vec::new(),
        rewritten: Vec::new(),
        diagnostics: corpus.diagnostics.clone(),
        source_parse_complete: corpus.diagnostics.is_empty(),
    };
    let mut attributions: BTreeMap<_, _> = attributions.into_iter().collect();
    for claim in &mut ledger.claims {
        let Some(attribution) = attributions.remove(&claim.id) else {
            continue;
        };
        count(&mut report.totals, claim, &attribution);
        count(
            report.by_family.entry(family(claim).into()).or_default(),
            claim,
            &attribution,
        );
        count(
            report.by_file.entry(claim.file.clone()).or_default(),
            claim,
            &attribution,
        );
        match attribution.comparison {
            Match::None => report.authored.push(claim.id.clone()),
            Match::Rewritten => report.rewritten.push(claim.id.clone()),
            Match::Exact => {}
        }
        claim.owner = match attribution.origin {
            Origin::EngineText => Owner::EngineFact,
            Origin::ShippedComment => Owner::ContentDerived,
            Origin::Authored => Owner::AuthoredText,
        };
        claim.provisional_owner = attribution.comparison == Match::Rewritten;
        claim.ownership_reason = match attribution.comparison {
            Match::Exact => {
                "Text matches a located source; this does not qualify documented behavior"
            }
            Match::Rewritten => "Near-text source candidate; requires review, not evidence",
            Match::None => "No matching text in supplied corpus; authored remainder, not evidence",
        }
        .into();
        claim.expected_method = (claim.owner == Owner::EngineFact).then(|| Method {
            ticket: "SDK-535".into(),
            name: "Engine declarations".into(),
        });
        claim.provenance = Some(attribution);
    }
    report
}
