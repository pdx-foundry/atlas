//! The game's own documentation logs (`script-docs`) against the snapshot.
//!
//! Each log gives lists of entries, such as command names or `name: description` pairs. A
//! property list compares only the names that both sides answer, so a missing answer never
//! reads as a disagreement.

use super::{
    Inputs, LogComparison,
    name_lists::{Engine, compare},
};
use crate::snapshot::SubjectKind;
use pdx_native::DeclaredTags;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The logs that the comparison reads, by file name.
pub const LOGS: [&str; 5] = [
    "effects.log",
    "triggers.log",
    "scopes.log",
    "localizations.log",
    "modifiers.log",
];

pub(super) fn compare_logs(engine: &Engine, inputs: &Inputs) -> Vec<LogComparison> {
    let scope_names = scope_names(engine);

    inputs
        .script_docs
        .iter()
        .filter_map(|(log, text)| {
            let lists = match log.as_str() {
                "effects.log" => commands(engine, SubjectKind::Effect, text, &scope_names),
                "triggers.log" => commands(engine, SubjectKind::Trigger, text, &scope_names),
                "scopes.log" => links(engine, text, &scope_names),
                "localizations.log" => localization(engine, text),
                "modifiers.log" => modifiers(engine, inputs, text),
                _ => return None,
            };

            Some(LogComparison {
                log: log.clone(),
                lists,
            })
        })
        .collect()
}

/// One documented entry: its header line and the lines up to its closing line.
struct Entry {
    name: String,
    description: String,
    body: Vec<String>,
    /// Text after each `label: ` line of the entry.
    labels: BTreeMap<String, String>,
}

/// Entries after the line that contains `start`, each closed by a line that starts with
/// `closing`.
fn entries(text: &str, start: &str, closing: &str, labels: &[&str]) -> Vec<Entry> {
    let lines = text
        .lines()
        .skip_while(|line| !line.contains(start))
        .skip(1);
    let mut entries = Vec::new();
    let mut buffer: Vec<&str> = Vec::new();

    for line in lines {
        buffer.push(line);

        if !line.starts_with(closing) {
            continue;
        }

        let body: Vec<_> = buffer
            .drain(..)
            .skip_while(|line| line.trim().is_empty())
            .collect();
        let Some((header, rest)) = body.split_first() else {
            continue;
        };
        let (name, description) = header.split_once(" - ").unwrap_or((header, ""));
        let mut entry = Entry {
            name: name.trim().into(),
            description: description.trim().into(),
            body: Vec::new(),
            labels: BTreeMap::new(),
        };

        for line in rest {
            match labels.iter().find_map(|label| {
                line.strip_prefix(&format!("{label}: "))
                    .map(|text| (label, text))
            }) {
                Some((label, text)) => {
                    entry.labels.insert((*label).into(), text.trim().into());
                }
                None => entry.body.push((*line).into()),
            }
        }

        while entry.body.last().is_some_and(|line| line.trim().is_empty()) {
            entry.body.pop();
        }

        entries.push(entry);
    }

    entries
}

/// Display name of each scope subject, longest first, for splitting a log's scope list: a
/// display name can contain a space (`pop job`).
fn scope_names(engine: &Engine) -> Vec<String> {
    let mut names: Vec<_> = engine
        .names(SubjectKind::Scope)
        .iter()
        .map(|name| display_name(name).to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    names.sort_by_key(|name| std::cmp::Reverse(name.split(' ').count()));
    names
}

fn display_name(subject_name: &str) -> &str {
    subject_name
        .split_once('/')
        .map_or(subject_name, |(name, _)| name)
}

/// A log's space-separated scope list as sorted display names; `all` stays `all`.
fn log_scopes(text: &str, scope_names: &[String]) -> String {
    let words: Vec<_> = text.split_whitespace().collect();
    let mut scopes = Vec::new();
    let mut index = 0;

    while index < words.len() {
        let matched = scope_names.iter().find(|name| {
            let parts: Vec<_> = name.split(' ').collect();

            words[index..].starts_with(&parts)
        });
        let length = matched.map_or(1, |name| name.split(' ').count());

        scopes.push(words[index..index + length].join(" "));
        index += length;
    }

    scopes.sort();
    scopes.join(" ")
}

/// A snapshot scope answer as sorted display names: `"any"` becomes `all`, `"various"` stays.
fn engine_scopes(answer: &Value) -> Option<String> {
    match answer {
        Value::String(text) if text == "any" => Some("all".into()),
        Value::String(text) => Some(text.clone()),
        Value::Array(ids) => {
            let mut scopes: Vec<_> = ids
                .iter()
                .map(|id| {
                    id.as_str()
                        .and_then(|id| id.strip_prefix("scope:"))
                        .map(|name| display_name(name).to_owned())
                })
                .collect::<Option<_>>()?;

            scopes.sort();
            Some(scopes.join(" "))
        }
        _ => None,
    }
}

/// Documentation text with each run of whitespace as one space: the log indents and pads lines
/// that the engine's string does not.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Compare one property over the names that both sides answer.
fn property(
    list: &str,
    log: &BTreeMap<String, String>,
    engine: impl Fn(&str) -> Option<String>,
) -> super::NameList {
    let mut engine_entries = BTreeSet::new();
    let mut log_entries = BTreeSet::new();

    for (name, logged) in log {
        if let Some(answer) = engine(name) {
            engine_entries.insert(format!("{name}: {answer}"));
            log_entries.insert(format!("{name}: {logged}"));
        }
    }

    compare(list, engine_entries, log_entries)
}

fn commands(
    engine: &Engine,
    kind: SubjectKind,
    text: &str,
    scope_names: &[String],
) -> Vec<super::NameList> {
    let entries = entries(
        text,
        "DOCUMENTATION ==",
        "Supported Scopes:",
        &["Supported Scopes"],
    );
    let documentation = |name: &str, field: &str| {
        engine
            .answer(kind, name, "documentation")
            .and_then(|answer| answer.get(field))
            .and_then(Value::as_str)
            .map(prose)
    };
    let logged = |value: fn(&Entry) -> String| -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|entry| (entry.name.clone(), value(entry)))
            .collect()
    };
    let scopes: BTreeMap<_, _> = entries
        .iter()
        .map(|entry| {
            let text = entry
                .labels
                .get("Supported Scopes")
                .cloned()
                .unwrap_or_default();

            (entry.name.clone(), log_scopes(&text, scope_names))
        })
        .collect();

    vec![
        compare(
            "names",
            engine.names(kind),
            entries.iter().map(|entry| entry.name.clone()).collect(),
        ),
        property(
            "descriptions",
            &logged(|entry| prose(&entry.description)),
            |name| documentation(name, "description"),
        ),
        property(
            "usage",
            &logged(|entry| prose(&entry.body.join("\n"))),
            |name| documentation(name, "usage"),
        ),
        property("supported_scopes", &scopes, |name| {
            engine
                .answer(kind, name, "declared_scopes")
                .and_then(engine_scopes)
        }),
    ]
}

fn links(engine: &Engine, text: &str, scope_names: &[String]) -> Vec<super::NameList> {
    let entries = entries(
        text,
        "Complete list of scope changes:",
        "Output Scope:",
        &["Supported Scopes", "Output Scope"],
    );
    let label = |label: &str| -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|entry| {
                let text = entry.labels.get(label).cloned().unwrap_or_default();

                (entry.name.clone(), log_scopes(&text, scope_names))
            })
            .collect()
    };
    let scopes = |property: &'static str| {
        move |name: &str| {
            engine
                .answer(SubjectKind::ScopeLink, name, property)
                .and_then(engine_scopes)
        }
    };

    vec![
        compare(
            "names",
            engine.names(SubjectKind::ScopeLink),
            entries.iter().map(|entry| entry.name.clone()).collect(),
        ),
        property(
            "supported_scopes",
            &label("Supported Scopes"),
            scopes("input_scopes"),
        ),
        property(
            "output_scope",
            &label("Output Scope"),
            scopes("output_scope"),
        ),
    ]
}

fn localization(engine: &Engine, text: &str) -> Vec<super::NameList> {
    let mut contexts = BTreeSet::new();
    let mut logged_commands = BTreeMap::<String, BTreeSet<String>>::new();
    let mut logged_links = BTreeMap::<String, BTreeSet<String>>::new();
    let mut context = None;
    let mut links = false;

    for line in text.lines() {
        if let Some(name) = line
            .strip_prefix("--")
            .and_then(|line| line.strip_suffix("--"))
        {
            contexts.insert(name.to_owned());
            context = Some(name.to_owned());
        } else if line.starts_with("Promotions") {
            links = true;
        } else if line.starts_with("Properties") {
            links = false;
        } else if let (Some(context), Some(item)) = (&context, line.strip_prefix(' ')) {
            let rows = if links {
                &mut logged_links
            } else {
                &mut logged_commands
            };

            rows.entry(item.trim().to_owned())
                .or_default()
                .insert(context.clone());
        }
    }

    let context_name = |id: &Value| {
        id.as_str()
            .and_then(|id| id.strip_prefix("localization_context:"))
            .map(str::to_owned)
    };
    let command_contexts = |name: &str| {
        engine
            .answer(SubjectKind::LocalizationCommand, name, "contexts")?
            .as_array()?
            .iter()
            .map(context_name)
            .collect::<Option<BTreeSet<_>>>()
    };
    let link_contexts = |name: &str| {
        let mut contexts = BTreeSet::new();

        for alternative in engine
            .answer(SubjectKind::LocalizationLink, name, "alternatives")?
            .as_array()?
        {
            for id in alternative.get("input_contexts")?.as_array()? {
                contexts.insert(context_name(id)?);
            }
        }

        Some(contexts)
    };

    vec![
        compare(
            "contexts",
            engine.names(SubjectKind::LocalizationContext),
            contexts,
        ),
        pairs("commands", &logged_commands, command_contexts),
        pairs("links", &logged_links, link_contexts),
    ]
}

/// `context.name` pairs for the names whose contexts both sides answer.
fn pairs(
    list: &str,
    logged: &BTreeMap<String, BTreeSet<String>>,
    engine: impl Fn(&str) -> Option<BTreeSet<String>>,
) -> super::NameList {
    let mut engine_pairs = BTreeSet::new();
    let mut log_pairs = BTreeSet::new();

    for (name, contexts) in logged {
        let Some(engine_contexts) = engine(name) else {
            continue;
        };

        engine_pairs.extend(
            engine_contexts
                .iter()
                .map(|context| format!("{context}.{name}")),
        );
        log_pairs.extend(contexts.iter().map(|context| format!("{context}.{name}")));
    }

    compare(list, engine_pairs, log_pairs)
}

fn modifiers(engine: &Engine, inputs: &Inputs, text: &str) -> Vec<super::NameList> {
    let logged: BTreeMap<String, String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("- "))
        .filter_map(|line| line.split_once(", Category: "))
        .map(|(name, tags)| (name.to_owned(), tags.to_owned()))
        .collect();
    let engine_tags: BTreeMap<String, Option<String>> = match inputs.loaded_modifiers {
        Some(loaded) => loaded
            .value
            .modifiers
            .iter()
            .map(|modifier| {
                let tags = match &modifier.category_tags {
                    DeclaredTags::Listed(tags) => Some(tags.join(", ")),
                    DeclaredTags::Unresolved => None,
                };

                (modifier.name.clone(), tags)
            })
            .collect(),
        None => engine
            .names(SubjectKind::Modifier)
            .into_iter()
            .map(|name| {
                let tags = engine
                    .answer(SubjectKind::Modifier, &name, "category_tags")
                    .and_then(|answer| serde_json::from_value::<Vec<String>>(answer.clone()).ok())
                    .map(|tags| tags.join(", "));

                (name, tags)
            })
            .collect(),
    };

    vec![
        compare(
            "names",
            engine_tags.keys().cloned().collect(),
            logged.keys().cloned().collect(),
        ),
        property("categories", &logged, |name| {
            engine_tags.get(name).cloned().flatten()
        }),
    ]
}
