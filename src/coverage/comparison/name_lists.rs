//! Config name lists, shipped define files and loaded modifier tags against the snapshot.

use super::{Inputs, NameList, TagComparison, TagDifference};
use crate::{
    ledger::{Claim, Ledger},
    snapshot::{Snapshot, SubjectKind},
};
use pdx_native::DeclaredTags;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The snapshot's answers, indexed for comparison.
pub(super) struct Engine<'a> {
    snapshot: &'a Snapshot,
    rules: BTreeMap<&'a str, &'a Value>,
}

impl<'a> Engine<'a> {
    pub(super) fn new(snapshot: &'a Snapshot) -> Self {
        let rules = snapshot
            .rules
            .iter()
            .filter(|rule| rule.conditions.is_empty())
            .map(|rule| (rule.id.as_str(), &rule.answer))
            .collect();

        Self { snapshot, rules }
    }

    /// Names of the subjects of `kind` that have an existence rule.
    pub(super) fn names(&self, kind: SubjectKind) -> BTreeSet<String> {
        self.snapshot
            .subjects
            .iter()
            .filter(|subject| subject.kind == kind)
            .filter(|subject| {
                self.rules
                    .contains_key(format!("{}#existence", subject.id).as_str())
            })
            .filter_map(|subject| subject.name.clone())
            .collect()
    }

    /// The answer of `property` for the subject of `kind` named `name`.
    pub(super) fn answer(&self, kind: SubjectKind, name: &str, property: &str) -> Option<&Value> {
        self.rules
            .get(format!("{}#{property}", kind.id(name)).as_str())
            .copied()
    }

    /// Script keywords of every scope type and scope group.
    fn scope_keywords(&self) -> BTreeSet<String> {
        let mut keywords = self.names(SubjectKind::ScopeGroup);

        for name in self.names(SubjectKind::Scope) {
            let answer = self.answer(SubjectKind::Scope, &name, "keywords");

            keywords.extend(
                answer
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
        }

        keywords
    }
}

pub(super) fn compare(list: &str, engine: BTreeSet<String>, config: BTreeSet<String>) -> NameList {
    NameList {
        list: list.into(),
        agree: engine.intersection(&config).cloned().collect(),
        engine_only: engine.difference(&config).cloned().collect(),
        config_only: config.difference(&engine).cloned().collect(),
    }
}

/// Each config name list against the snapshot's names.
pub(super) fn config_lists(ledger: &Ledger, engine: &Engine, inputs: &Inputs) -> Vec<NameList> {
    let mut modifiers = engine.names(SubjectKind::Modifier);

    if let Some(loaded) = inputs.loaded_modifiers {
        modifiers.extend(
            loaded
                .value
                .modifiers
                .iter()
                .map(|modifier| modifier.name.clone()),
        );
    }

    vec![
        compare(
            "effects",
            engine.names(SubjectKind::Effect),
            config_names(ledger, |claim| command_name(claim, "effect")),
        ),
        compare(
            "triggers",
            engine.names(SubjectKind::Trigger),
            config_names(ledger, |claim| command_name(claim, "trigger")),
        ),
        compare(
            "modifiers",
            modifiers,
            config_names(ledger, |claim| {
                keyed(claim, "modifiers.cwt", "modifiers", "declaration_existence")
            }),
        ),
        compare(
            "modifier_categories",
            engine.names(SubjectKind::ModifierCategory),
            config_names(ledger, |claim| {
                keyed(
                    claim,
                    "modifier_categories.cwt",
                    "modifier_categories",
                    "declaration_existence",
                )
            }),
        ),
        compare(
            "scope_keywords",
            engine.scope_keywords(),
            config_names(ledger, |claim| match claim.subject.as_slice() {
                [root, _, aliases, _]
                    if claim.file == "scopes.cwt"
                        && root == "scopes"
                        && aliases == "aliases"
                        && claim.property == "scope_alias" =>
                {
                    Some(claim.config_answer.trim_matches('"').to_owned())
                }
                _ => None,
            }),
        ),
        compare(
            "scope_links",
            engine.names(SubjectKind::ScopeLink),
            config_names(ledger, |claim| {
                keyed(claim, "links.cwt", "links", "declaration_existence")
            }),
        ),
        compare(
            "localisation_commands",
            engine.names(SubjectKind::LocalizationCommand),
            config_names(ledger, |claim| {
                keyed(
                    claim,
                    "localisation.cwt",
                    "localisation_commands",
                    "field_existence",
                )
            }),
        ),
        compare(
            "localisation_links",
            engine.names(SubjectKind::LocalizationLink),
            config_names(ledger, |claim| {
                keyed(
                    claim,
                    "localisation.cwt",
                    "localisation_promotions",
                    "field_existence",
                )
                .or_else(|| {
                    keyed(
                        claim,
                        "localisation_links.cwt",
                        "localisation_links",
                        "declaration_existence",
                    )
                })
            }),
        ),
        compare(
            "on_actions",
            engine.names(SubjectKind::OnAction),
            config_names(ledger, |claim| match claim.subject.as_slice() {
                [root, _]
                    if claim.file == "on_actions.cwt"
                        && root == "on_actions"
                        && claim.property == "value_form" =>
                {
                    Some(claim.config_answer.trim_matches('"').to_owned())
                }
                _ => None,
            }),
        ),
        compare(
            "game_rules",
            engine.names(SubjectKind::GameRule),
            config_names(ledger, |claim| {
                keyed(claim, "game_rules.cwt", "game_rules", "field_existence")
            }),
        ),
        compare(
            "defines",
            engine.names(SubjectKind::Define),
            config_names(ledger, |claim| match claim.subject.as_slice() {
                [root, namespace, name]
                    if claim.file.starts_with("common/defines/")
                        && root == "defines"
                        && claim.property == "field_existence" =>
                {
                    Some(format!("{namespace}.{name}"))
                }
                _ => None,
            }),
        ),
    ]
}

fn config_names(ledger: &Ledger, name: impl Fn(&Claim) -> Option<String>) -> BTreeSet<String> {
    ledger
        .claims
        .iter()
        .filter(|claim| claim.conditions.is_empty())
        .filter_map(name)
        .collect()
}

fn keyed(claim: &Claim, file: &str, root: &str, property: &str) -> Option<String> {
    match claim.subject.as_slice() {
        [first, name] if claim.file == file && first == root && claim.property == property => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn command_name(claim: &Claim, kind: &str) -> Option<String> {
    let [segment] = claim.subject.as_slice() else {
        return None;
    };
    let name = segment
        .strip_prefix(&format!("alias[{kind}:"))?
        .strip_suffix(']')?;

    (claim.property == "command_existence" && !name.contains('<')).then(|| name.to_owned())
}

/// The define names of the shipped define files against the snapshot's defines.
pub(super) fn define_files(engine: &Engine, inputs: &Inputs) -> Result<Option<NameList>, String> {
    if inputs.define_files.is_empty() {
        return Ok(None);
    }

    let mut shipped = BTreeSet::new();

    for (file, text) in &inputs.define_files {
        let document = pdxscript::cwt::parse(text, file);

        if let Some(diagnostic) = document.diagnostics.first() {
            return Err(format!(
                "{file} could not be read as script: {}",
                diagnostic.message
            ));
        }

        for namespace in &document.nodes {
            let (Some(key), pdxscript::cwt::Value::Block { nodes, .. }) =
                (&namespace.key, &namespace.value)
            else {
                continue;
            };

            shipped.extend(
                nodes
                    .iter()
                    .filter_map(|define| define.key.as_ref())
                    .map(|name| format!("{}.{}", key.text, name.text)),
            );
        }
    }

    Ok(Some(compare(
        "shipped_defines",
        engine.names(SubjectKind::Define),
        shipped,
    )))
}

/// The static category tags of each declared modifier against its tags in the loaded table.
pub(super) fn modifier_tags(engine: &Engine, inputs: &Inputs) -> Option<TagComparison> {
    let loaded: BTreeMap<_, _> = inputs
        .loaded_modifiers?
        .value
        .modifiers
        .iter()
        .map(|modifier| (modifier.name.as_str(), &modifier.category_tags))
        .collect();
    let mut comparison = TagComparison {
        agree: 0,
        different: Vec::new(),
    };

    for name in engine.names(SubjectKind::Modifier) {
        let declared = engine
            .answer(SubjectKind::Modifier, &name, "category_tags")
            .and_then(|answer| serde_json::from_value::<Vec<String>>(answer.clone()).ok());
        let loaded = match loaded.get(name.as_str()) {
            Some(DeclaredTags::Listed(tags)) => Some(tags.clone()),
            _ => None,
        };

        if declared.is_some() && loaded == declared {
            comparison.agree += 1;
        } else {
            comparison.different.push(TagDifference {
                name,
                declared,
                loaded,
            });
        }
    }

    Some(comparison)
}
