use super::{classify, *};
use pdxscript::{
    Span,
    cwt::{self, Node, Value},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Builder<'a> {
    file: &'a str,
    source: &'a str,
    claims: Vec<Claim>,
    diagnostics: Vec<Diagnostic>,
    occurrences: BTreeMap<String, usize>,
    lines: BTreeMap<usize, Line>,
}
struct Question<'a> {
    subject: &'a [String],
    property: &'a str,
    answer: String,
    span: Span,
    owner: Owner,
    reason: &'a str,
    provisional: bool,
}
impl Builder<'_> {
    fn claim(&mut self, question: Question<'_>) -> String {
        let Question {
            subject,
            property,
            answer,
            span,
            owner,
            reason,
            provisional,
        } = question;
        let conditions = subject
            .iter()
            .filter(|s| s.starts_with("subtype["))
            .cloned()
            .collect::<Vec<_>>();
        let identity = serde_json::to_vec(&(self.file, subject, property, &conditions))
            .expect("serializable identity");
        let question = format!("question:{}", hash(&identity));
        let occurrence = self.occurrences.entry(question.clone()).or_default();
        *occurrence += 1;
        let id = format!("{question}:{}", *occurrence);
        self.claims.push(Claim {
            id: id.clone(),
            question,
            file: self.file.into(),
            subject: subject.to_vec(),
            property: property.into(),
            conditions,
            config_answer: answer.clone(),
            span,
            source_text: self.source.get(span.start..span.end).unwrap_or("").into(),
            owner,
            ownership_reason: reason.into(),
            provisional_owner: provisional,
            provenance: None,
            expected_method: (owner == Owner::EngineFact)
                .then(|| classify::route(self.file, subject, property, &answer)),
        });
        self.link(span, &id);
        id
    }
    fn link(&mut self, span: Span, id: &str) {
        for number in span.line..=span.end_line {
            if let Some(line) = self.lines.get_mut(&number) {
                line.claims.push(id.into());
            }
        }
    }
    fn diagnostic(&mut self, kind: &str, span: Span, message: String) {
        let id = format!(
            "diagnostic:{}:{}:{}",
            self.file,
            span.start,
            self.diagnostics.len() + 1
        );
        for number in span.line..=span.end_line {
            if let Some(line) = self.lines.get_mut(&number) {
                line.diagnostics.push(id.clone());
            }
        }
        self.diagnostics.push(Diagnostic {
            id,
            file: self.file.into(),
            kind: kind.into(),
            span,
            message,
        });
    }
    fn cardinality(&mut self, subject: &[String], value: &str, span: Span) -> bool {
        let range = value.split('#').next().unwrap_or(value).trim();
        let Some((minimum, maximum)) = range.split_once("..") else {
            return false;
        };
        for (bound, hard_property, soft_property) in [
            (minimum, "cardinality_minimum", "soft_cardinality_minimum"),
            (maximum, "cardinality_maximum", "soft_cardinality_maximum"),
        ] {
            let soft = bound.starts_with('~');
            self.claim(Question {
                subject,
                property: if soft { soft_property } else { hard_property },
                answer: bound.trim_start_matches('~').into(),
                span,
                owner: if soft {
                    Owner::ConsumerPolicy
                } else {
                    Owner::EngineFact
                },
                reason: if soft {
                    "Soft occurrence recommendation belongs to the consumer"
                } else {
                    "Hard occurrence bound requires engine evidence"
                },
                provisional: false,
            });
        }
        true
    }
    fn nodes(&mut self, nodes: &[Node], path: &[String]) {
        let mut bare_index = 0;
        for node in nodes {
            let mut subject = path.to_vec();
            let key = node.key.as_ref().map(|s| s.text.as_str());
            if let Some(key) = key {
                subject.push(key.into());
            } else {
                bare_index += 1;
                subject.push(format!("$item:{bare_index}"));
            }
            let answer = match &node.value {
                Value::Scalar(s) => s.text.clone(),
                Value::Block { .. } => "block".into(),
            };
            let (property, owner, reason) = classify::node(self.file, path, key, &answer);
            let header_span = match &node.value {
                Value::Scalar(_) => node.span,
                Value::Block { span, .. } => Span {
                    end: span.start + 1,
                    end_line: span.line,
                    ..node.span
                },
            };
            let id = self.claim(Question {
                subject: &subject,
                property,
                answer: if property.ends_with("existence") {
                    "present".into()
                } else {
                    answer.clone()
                },
                span: header_span,
                owner,
                reason,
                provisional: false,
            });
            if let Value::Block { span, .. } = &node.value {
                self.link(
                    Span {
                        start: span.end.saturating_sub(1),
                        end: span.end,
                        line: span.end_line,
                        end_line: span.end_line,
                    },
                    &id,
                );
            }
            if property == "field_existence"
                || property == "command_existence"
                || (property == "alias_factoring"
                    && !key.is_some_and(|k| k.starts_with("alias_name[")))
            {
                self.claim(Question {
                    subject: &subject,
                    property: "value_form",
                    answer,
                    span: header_span,
                    owner: Owner::EngineFact,
                    reason: "Accepted value form is distinct from field/command existence",
                    provisional: false,
                });
            }
            let mut annotations = node.annotations.iter().peekable();
            while let Some(annotation) = annotations.next() {
                if annotation.hashes >= 3 {
                    let (owner, reason) = classify::documentation_owner(self.file);
                    let mut text = annotation.text.clone();
                    let mut span = annotation.span;
                    while annotations
                        .peek()
                        .is_some_and(|a| a.hashes >= 3 && a.span.line == span.end_line + 1)
                    {
                        let next = annotations.next().expect("consecutive documentation");
                        text.push('\n');
                        text.push_str(&next.text);
                        span.end = next.span.end;
                        span.end_line = next.span.end_line;
                    }
                    self.claim(Question {
                        subject: &subject,
                        property: "documentation",
                        answer: text,
                        span,
                        owner,
                        reason,
                        provisional: true,
                    });
                    continue;
                }
                let text = annotation.text.trim();
                let name = text
                    .split(|c: char| c.is_whitespace() || c == '=')
                    .next()
                    .unwrap_or("");
                let value = text[name.len()..]
                    .trim()
                    .strip_prefix('=')
                    .or_else(|| text[name.len()..].trim().strip_prefix("<>"))
                    .unwrap_or(text[name.len()..].trim())
                    .trim();
                if let Some(problem) =
                    classify::annotation_problem(name, text[name.len()..].trim(), value)
                {
                    self.diagnostic("malformed-annotation", annotation.span, problem);
                }
                let mut annotation_subject = subject.clone();
                annotation_subject.push(format!("$annotation:{name}"));
                if name == "cardinality"
                    && self.cardinality(&annotation_subject, value, annotation.span)
                {
                    continue;
                }
                if let Some((property, owner, reason)) = classify::annotation_property(name, value)
                {
                    self.claim(Question {
                        subject: &annotation_subject,
                        property,
                        answer: format!("{name}: {value}"),
                        span: annotation.span,
                        owner,
                        reason,
                        provisional: false,
                    });
                } else {
                    self.claim(Question {
                        subject: &subject,
                        property: "uninterpreted_annotation",
                        answer: text.into(),
                        span: annotation.span,
                        owner: Owner::ConsumerPolicy,
                        reason: "Unrecognized config metadata; no game assertion is inferred",
                        provisional: true,
                    });
                    self.diagnostic(
                        "unknown-annotation",
                        annotation.span,
                        format!("Uninterpreted ## annotation: {text}"),
                    );
                }
            }
            if let Value::Block { nodes, .. } = &node.value {
                self.nodes(nodes, &subject);
            }
        }
    }
}
fn meaningful_lines(source: &str) -> BTreeMap<usize, Line> {
    // Track quotes across physical lines: a '#' within quoted text is not a comment.
    let mut quoted = false;
    let mut escaped = false;
    let mut result = BTreeMap::new();
    for (index, text) in source.split_inclusive('\n').enumerate() {
        let trimmed = text.trim_start_matches([' ', '\t', '\r', '\n', '\u{feff}']);
        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        let meaningful =
            quoted || (!trimmed.is_empty() && (hashes == 0 || (2..=4).contains(&hashes)));
        if meaningful {
            result.insert(
                index + 1,
                Line {
                    number: index + 1,
                    claims: Vec::new(),
                    diagnostics: Vec::new(),
                },
            );
        }
        for ch in text.chars() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quoted = false;
                }
            } else if ch == '#' {
                break;
            } else if ch == '"' {
                quoted = true;
            }
        }
    }
    result
}
/// Builds a deterministic ledger from caller-supplied relative paths and source strings.
/// Unknown syntax remains in diagnostics and line accounting; no snapshot can hide it.
pub fn inventory(sources: &Sources) -> Ledger {
    let mut files = Vec::new();
    let mut claims = Vec::new();
    let mut diagnostics = Vec::new();
    for (file, source) in sources {
        let mut builder = Builder {
            file,
            source,
            claims: Vec::new(),
            diagnostics: Vec::new(),
            occurrences: BTreeMap::new(),
            lines: meaningful_lines(source),
        };
        let document = cwt::parse(source, file);
        for diagnostic in document.diagnostics {
            builder.diagnostic(&diagnostic.kind, diagnostic.span, diagnostic.message);
        }
        builder.nodes(&document.nodes, &[]);
        let missing: Vec<_> = builder
            .lines
            .values()
            .filter(|line| line.claims.is_empty() && line.diagnostics.is_empty())
            .map(|l| l.number)
            .collect();
        for number in missing {
            let start = source
                .split_inclusive('\n')
                .take(number - 1)
                .map(str::len)
                .sum();
            let text = source.split_inclusive('\n').nth(number - 1).unwrap_or("");
            builder.diagnostic(
                "unaccounted-source",
                Span {
                    start,
                    end: start + text.len(),
                    line: number,
                    end_line: number,
                },
                text.trim_end().into(),
            );
        }
        for line in builder.lines.values_mut() {
            line.claims.sort();
            line.claims.dedup();
            line.diagnostics.sort();
            line.diagnostics.dedup();
        }
        files.push(File {
            path: file.clone(),
            sha256: hash(source.as_bytes()),
            physical_lines: source.split_inclusive('\n').count(),
            lines: builder.lines.into_values().collect(),
        });
        claims.extend(builder.claims);
        diagnostics.extend(builder.diagnostics);
    }
    claims.sort_by(|a, b| a.id.cmp(&b.id));
    diagnostics.sort_by(|a, b| (&a.file, a.span.start, &a.id).cmp(&(&b.file, b.span.start, &b.id)));
    let identity: Vec<_> = files.iter().map(|f| (&f.path, &f.sha256)).collect();
    Ledger {
        format_version: 1,
        config_sha256: hash(&serde_json::to_vec(&identity).expect("file manifest")),
        files,
        claims,
        diagnostics,
    }
}
