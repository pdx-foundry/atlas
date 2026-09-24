//! Join config language questions to language subjects of the snapshot.
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
    let scopes = scope_join(ledger, snapshot);
    let on_actions = on_action_names(ledger);
    let mut index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for claim in ledger
        .claims
        .iter()
        .filter(|claim| claim.conditions.is_empty())
    {
        if let Some(record) = record(claim, &scopes, &on_actions) {
            index
                .entry(record)
                .or_default()
                .insert(claim.question.clone());
        }
    }

    index
}

fn record(
    claim: &Claim,
    scopes: &BTreeMap<&str, (SubjectKind, String)>,
    on_actions: &BTreeMap<&str, String>,
) -> Option<String> {
    let subject: Vec<&str> = claim.subject.iter().map(String::as_str).collect();

    if let Some((kind, name)) = subject.first().and_then(|first| command(first)) {
        return command_record(claim, kind, name, &subject[1..]);
    }

    let id = |kind: SubjectKind, name: &str, property: &str| {
        Some(format!("{}#{property}", kind.id(name)))
    };
    let file = claim.file.as_str();
    let property = claim.property.as_str();

    match (file, subject.as_slice(), property) {
        ("modifiers.cwt", ["modifiers", name], "declaration_existence") => {
            id(SubjectKind::Modifier, name, "existence")
        }
        ("modifiers.cwt", ["modifiers", name, _], "modifier_category") => {
            id(SubjectKind::Modifier, name, "category_tags")
        }
        ("modifier_categories.cwt", ["modifier_categories", name], "declaration_existence") => {
            id(SubjectKind::ModifierCategory, name, "existence")
        }
        (
            "modifier_categories.cwt",
            ["modifier_categories", name, "supported_scopes", ..],
            "declared_scopes",
        ) => id(SubjectKind::ModifierCategory, name, "supported_scopes"),
        ("scopes.cwt", ["scopes", name], "declaration_existence") => {
            let (kind, native) = scopes.get(name)?;

            id(*kind, native, "existence")
        }
        ("scopes.cwt", ["scopes", name, "is_subscope_of"], "declared_scopes") => {
            match scopes.get(name)? {
                (SubjectKind::Scope, native) => id(SubjectKind::Scope, native, "groups"),
                _ => None,
            }
        }
        ("links.cwt", ["links", name], "declaration_existence") => {
            id(SubjectKind::ScopeLink, name, "existence")
        }
        ("links.cwt", ["links", name, "input_scopes", ..], "declared_scopes") => {
            id(SubjectKind::ScopeLink, name, "input_scopes")
        }
        ("links.cwt", ["links", name, "output_scope"], "declared_scopes") => {
            id(SubjectKind::ScopeLink, name, "output_scope")
        }
        (
            "links.cwt",
            ["links", name, "prefix" | "from_data"],
            "field_existence" | "value_form",
        ) => id(SubjectKind::ScopeLink, name, "data"),
        ("links.cwt", ["links", name, "data_source"], "field_existence" | "value_form") => {
            id(SubjectKind::ScopeLink, name, "data_source")
        }
        ("localisation.cwt", ["localisation_commands", name], "field_existence") => {
            id(SubjectKind::LocalizationCommand, name, "existence")
        }
        ("localisation.cwt", ["localisation_commands", name, _], "value_form") => {
            id(SubjectKind::LocalizationCommand, name, "scopes")
        }
        ("localisation.cwt", ["localisation_promotions", name], "field_existence")
        | ("localisation_links.cwt", ["localisation_links", name], "declaration_existence") => {
            id(SubjectKind::LocalizationLink, name, "existence")
        }
        ("localisation.cwt", ["localisation_promotions", name, _], "value_form")
        | (
            "localisation_links.cwt",
            ["localisation_links", name, "input_scopes", ..],
            "declared_scopes",
        ) => id(SubjectKind::LocalizationLink, name, "input_scopes"),
        ("on_actions.cwt", ["on_actions", item], "value_form") => {
            id(SubjectKind::OnAction, on_actions.get(item)?, "existence")
        }
        ("on_actions.cwt", ["on_actions", item, "$annotation:replace_scopes"], "scope_context") => {
            id(SubjectKind::OnAction, on_actions.get(item)?, "entry_scopes")
        }
        ("game_rules.cwt", ["game_rules", name], "field_existence") => {
            id(SubjectKind::GameRule, name, "existence")
        }
        ("game_rules.cwt", ["game_rules", name, "$annotation:replace_scopes"], "scope_context") => {
            id(SubjectKind::GameRule, name, "entry_scopes")
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

            id(SubjectKind::Define, &define, property)
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

fn command_record(claim: &Claim, kind: SubjectKind, name: &str, rest: &[&str]) -> Option<String> {
    let property = match (rest, claim.property.as_str()) {
        ([], "command_existence") => "existence",
        ([], "documentation") => "documentation",
        (["$annotation:scopes"], "declared_scopes") => "declared_scopes",
        (_, "scope_context") => "scope_context",
        ([], "value_form") | ([_, ..], _) if claim.owner == Owner::EngineFact => "arguments",
        _ => return None,
    };

    Some(format!("{}#{property}", kind.id(name)))
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

    for claim in ledger
        .claims
        .iter()
        .filter(|claim| claim.file == "scopes.cwt" && claim.property == "scope_alias")
    {
        if let [root, name, alias, _] = claim.subject.as_slice()
            && root == "scopes"
            && alias == "aliases"
        {
            aliases
                .entry(name.as_str())
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
