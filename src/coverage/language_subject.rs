//! One classification of config claims into language subjects and questions.
//!
//! The join names the snapshot record that answers each config question. It reads the config's
//! structure and, where CWT identifies a subject only by its value (an on_action name, a scope
//! alias), that value; it never compares the config's answer with Atlas's.

use crate::{
    ledger::{Claim, Ledger, Owner},
    snapshot::{Snapshot, SubjectKind},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Config questions answered by each snapshot rule or gap identity.
pub(super) fn index(ledger: &Ledger, snapshot: &Snapshot) -> BTreeMap<String, BTreeSet<String>> {
    let mut index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (claim, subject) in classify(ledger, snapshot) {
        index
            .entry(subject.rule_id())
            .or_default()
            .insert(claim.question.clone());
    }

    index
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LanguageSubject {
    pub kind: SubjectKind,
    pub name: String,
    pub question: &'static str,
}

impl LanguageSubject {
    fn rule_id(&self) -> String {
        format!("{}#{}", self.kind.id(&self.name), self.question)
    }
}

/// Classify all language claims for a ledger. Scope matching needs snapshot keywords;
/// unmatched or ambiguous scopes have no snapshot subject.
pub(super) fn classify<'a>(
    ledger: &'a Ledger,
    snapshot: &Snapshot,
) -> Vec<(&'a Claim, LanguageSubject)> {
    let scopes = scope_join(ledger, snapshot);
    let on_actions = on_action_names(ledger);
    ledger
        .claims
        .iter()
        .filter(|claim| claim.conditions.is_empty())
        .filter_map(|claim| record(claim, &scopes, &on_actions).map(|subject| (claim, subject)))
        .collect()
}

/// Config keyword spelling is a name-list entry even if no unique Native scope matches it.
pub(super) fn scope_keyword(claim: &Claim) -> Option<String> {
    match claim.subject.as_slice() {
        [root, _, aliases, _]
            if claim.file == "scopes.cwt"
                && root == "scopes"
                && aliases == "aliases"
                && claim.property == "scope_alias" =>
        {
            Some(claim.config_answer.trim_matches('"').to_owned())
        }
        _ => None,
    }
}

fn record(
    claim: &Claim,
    scopes: &BTreeMap<&str, (SubjectKind, String)>,
    on_actions: &BTreeMap<&str, String>,
) -> Option<LanguageSubject> {
    let subject: Vec<&str> = claim.subject.iter().map(String::as_str).collect();

    if let Some((kind, name)) = subject.first().and_then(|first| command(first)) {
        return command_record(claim, kind, name, &subject[1..]);
    }

    let language_subject = |kind: SubjectKind, name: &str, question: &'static str| {
        Some(LanguageSubject {
            kind,
            name: name.to_owned(),
            question,
        })
    };
    let file = claim.file.as_str();
    let property = claim.property.as_str();

    match (file, subject.as_slice(), property) {
        ("modifiers.cwt", ["modifiers", name], "declaration_existence") => {
            language_subject(SubjectKind::Modifier, name, "existence")
        }
        ("modifiers.cwt", ["modifiers", name, _], "modifier_category") => {
            language_subject(SubjectKind::Modifier, name, "category_tags")
        }
        ("modifier_categories.cwt", ["modifier_categories", name], "declaration_existence") => {
            language_subject(SubjectKind::ModifierCategory, name, "existence")
        }
        (
            "modifier_categories.cwt",
            ["modifier_categories", name, "supported_scopes", ..],
            "declared_scopes",
        ) => language_subject(SubjectKind::ModifierCategory, name, "supported_scopes"),
        ("scopes.cwt", ["scopes", name], "declaration_existence") => {
            let (kind, native) = scopes.get(name)?;

            language_subject(*kind, native, "existence")
        }
        ("scopes.cwt", ["scopes", name, "is_subscope_of"], "declared_scopes") => {
            match scopes.get(name)? {
                (SubjectKind::Scope, native) => {
                    language_subject(SubjectKind::Scope, native, "groups")
                }
                _ => None,
            }
        }
        ("links.cwt", ["links", name], "declaration_existence") => {
            language_subject(SubjectKind::ScopeLink, name, "existence")
        }
        ("links.cwt", ["links", name, "input_scopes", ..], "declared_scopes") => {
            language_subject(SubjectKind::ScopeLink, name, "input_scopes")
        }
        ("links.cwt", ["links", name, "output_scope"], "declared_scopes") => {
            language_subject(SubjectKind::ScopeLink, name, "output_scope")
        }
        (
            "links.cwt",
            ["links", name, "prefix" | "from_data"],
            "field_existence" | "value_form",
        ) => language_subject(SubjectKind::ScopeLink, name, "data"),
        ("links.cwt", ["links", name, "data_source"], "field_existence" | "value_form") => {
            language_subject(SubjectKind::ScopeLink, name, "data_source")
        }
        ("localisation.cwt", ["localisation_commands", name], "field_existence") => {
            language_subject(SubjectKind::LocalizationCommand, name, "existence")
        }
        ("localisation.cwt", ["localisation_commands", name, _], "value_form") => {
            language_subject(SubjectKind::LocalizationCommand, name, "scopes")
        }
        ("localisation.cwt", ["localisation_promotions", name], "field_existence")
        | ("localisation_links.cwt", ["localisation_links", name], "declaration_existence") => {
            language_subject(SubjectKind::LocalizationLink, name, "existence")
        }
        ("localisation.cwt", ["localisation_promotions", name, _], "value_form")
        | (
            "localisation_links.cwt",
            ["localisation_links", name, "input_scopes", ..],
            "declared_scopes",
        ) => language_subject(SubjectKind::LocalizationLink, name, "input_scopes"),
        ("on_actions.cwt", ["on_actions", item], "value_form") => {
            language_subject(SubjectKind::OnAction, on_actions.get(item)?, "existence")
        }
        ("on_actions.cwt", ["on_actions", item, "$annotation:replace_scopes"], "scope_context") => {
            language_subject(SubjectKind::OnAction, on_actions.get(item)?, "entry_scopes")
        }
        ("game_rules.cwt", ["game_rules", name], "field_existence") => {
            language_subject(SubjectKind::GameRule, name, "existence")
        }
        ("game_rules.cwt", ["game_rules", name, "$annotation:replace_scopes"], "scope_context") => {
            language_subject(SubjectKind::GameRule, name, "entry_scopes")
        }
        (_, ["defines", namespace, name], "field_existence" | "value_form")
            if file.starts_with("common/defines/") =>
        {
            let define = format!("{namespace}.{name}");
            let property = if property == "value_form" {
                "value_type"
            } else {
                "existence"
            };

            language_subject(SubjectKind::Define, &define, property)
        }
        _ => None,
    }
}

/// `alias[effect:name]` or `alias[trigger:name]` for a command, not an alias over content
/// references such as `alias[effect:<scripted_effect>]`.
fn command(segment: &str) -> Option<(SubjectKind, &str)> {
    let inner = segment.strip_prefix("alias[")?.strip_suffix(']')?;
    let (kind, name) = inner.split_once(':')?;

    if name.contains('<') {
        return None;
    }

    match kind {
        "effect" => Some((SubjectKind::Effect, name)),
        "trigger" => Some((SubjectKind::Trigger, name)),
        _ => None,
    }
}

fn command_record(
    claim: &Claim,
    kind: SubjectKind,
    name: &str,
    rest: &[&str],
) -> Option<LanguageSubject> {
    let property = match (rest, claim.property.as_str()) {
        ([], "command_existence") => "existence",
        ([], "documentation") => "documentation",
        (["$annotation:scopes"], "declared_scopes") => "declared_scopes",
        (_, "scope_context") => "scope_context",
        ([], "value_form") | ([_, ..], _) if claim.owner == Owner::EngineFact => "arguments",
        _ => return None,
    };

    Some(LanguageSubject {
        kind,
        name: name.into(),
        question: property,
    })
}

/// The on_action name of each bare `on_actions` item, which CWT identifies only by position.
fn on_action_names(ledger: &Ledger) -> BTreeMap<&str, String> {
    ledger
        .claims
        .iter()
        .filter(|claim| claim.file == "on_actions.cwt" && claim.property == "value_form")
        .filter_map(|claim| match claim.subject.as_slice() {
            [root, item] if root == "on_actions" => Some((
                item.as_str(),
                claim.config_answer.trim_matches('"').to_owned(),
            )),
            _ => None,
        })
        .collect()
}

/// The Native scope type or group that each config scope names, joined by its declared aliases
/// against Native's keywords. A config scope that matches no subject, or several, stays unjoined.
fn scope_join<'a>(
    ledger: &'a Ledger,
    snapshot: &Snapshot,
) -> BTreeMap<&'a str, (SubjectKind, String)> {
    let mut keywords: BTreeMap<String, BTreeSet<(SubjectKind, String)>> = BTreeMap::new();

    for rule in snapshot
        .rules
        .iter()
        .filter(|rule| rule.property == "keywords")
    {
        let Some(name) = rule.subject.strip_prefix("scope:") else {
            continue;
        };

        for keyword in rule
            .answer
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            keywords
                .entry(keyword.into())
                .or_default()
                .insert((SubjectKind::Scope, name.into()));
        }
    }

    for subject in snapshot
        .subjects
        .iter()
        .filter(|subject| subject.kind == SubjectKind::ScopeGroup)
    {
        let name = subject.name.clone().unwrap_or_default();

        keywords
            .entry(name.clone())
            .or_default()
            .insert((SubjectKind::ScopeGroup, name));
    }

    let mut aliases: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();

    for claim in &ledger.claims {
        if scope_keyword(claim).is_some() {
            aliases
                .entry(claim.subject[1].as_str())
                .or_default()
                .insert(claim.config_answer.trim_matches('"'));
        }
    }

    aliases
        .into_iter()
        .filter_map(|(name, aliases)| {
            let matched: BTreeSet<_> = aliases
                .iter()
                .filter_map(|alias| keywords.get(*alias))
                .flatten()
                .collect();

            match matched.into_iter().collect::<Vec<_>>().as_slice() {
                [only] => Some((name, (*only).clone())),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger;

    #[test]
    fn classifies_command_arguments_on_actions_scope_aliases_and_defines() {
        let sources = [
            (
                "effects.cwt".into(),
                "alias[effect:add_building] = { building = <building> }".into(),
            ),
            (
                "on_actions.cwt".into(),
                "on_actions = { on_game_start }".into(),
            ),
            (
                "scopes.cwt".into(),
                "scopes = { System = { aliases = { galacticobject system } } }".into(),
            ),
            (
                "common/defines/00_defines.cwt".into(),
                "defines = { NGameplay = { LOGISTIC_CEILING_MIN = int } }".into(),
            ),
        ]
        .into();
        let ledger = ledger::inventory(&sources);
        assert!(ledger.diagnostics.is_empty(), "{:?}", ledger.diagnostics);
        let on_actions = on_action_names(&ledger);
        let scopes = BTreeMap::from([("System", (SubjectKind::Scope, "galactic_object".into()))]);

        let find = |file: &str, property: &str, subject: &[&str]| {
            ledger
                .claims
                .iter()
                .find(|claim| {
                    claim.file == file && claim.property == property && claim.subject == subject
                })
                .unwrap()
        };
        let classified = |claim| record(claim, &scopes, &on_actions).unwrap();

        assert_eq!(
            classified(find(
                "effects.cwt",
                "field_existence",
                &["alias[effect:add_building]", "building"]
            )),
            LanguageSubject {
                kind: SubjectKind::Effect,
                name: "add_building".into(),
                question: "arguments"
            }
        );
        assert_eq!(
            classified(find(
                "on_actions.cwt",
                "value_form",
                &["on_actions", "$item:1"]
            )),
            LanguageSubject {
                kind: SubjectKind::OnAction,
                name: "on_game_start".into(),
                question: "existence"
            }
        );
        assert_eq!(
            classified(find(
                "scopes.cwt",
                "declaration_existence",
                &["scopes", "System"]
            )),
            LanguageSubject {
                kind: SubjectKind::Scope,
                name: "galactic_object".into(),
                question: "existence"
            }
        );
        let aliases = ledger
            .claims
            .iter()
            .filter_map(scope_keyword)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            aliases,
            BTreeSet::from(["galacticobject".into(), "system".into()])
        );
        assert_eq!(
            classified(find(
                "common/defines/00_defines.cwt",
                "value_form",
                &["defines", "NGameplay", "LOGISTIC_CEILING_MIN"]
            )),
            LanguageSubject {
                kind: SubjectKind::Define,
                name: "NGameplay.LOGISTIC_CEILING_MIN".into(),
                question: "value_type"
            }
        );
    }
}
