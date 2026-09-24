//! Config name lists, shipped define files and loaded modifier tags against the snapshot.

use super::{Inputs, NameList, TagComparison, TagDifference};
use crate::{
    coverage::language_subject,
    ledger::Ledger,
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
    let classified = language_subject::classify(ledger, engine.snapshot);
    let config_names = |kind| {
        classified
            .iter()
            .filter(|(_, subject)| subject.kind == kind && subject.question == "existence")
            .map(|(_, subject)| subject.name.clone())
            .collect()
    };

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
            config_names(SubjectKind::Effect),
        ),
        compare(
            "triggers",
            engine.names(SubjectKind::Trigger),
            config_names(SubjectKind::Trigger),
        ),
        compare("modifiers", modifiers, config_names(SubjectKind::Modifier)),
        compare(
            "modifier_categories",
            engine.names(SubjectKind::ModifierCategory),
            config_names(SubjectKind::ModifierCategory),
        ),
        compare(
            "scope_keywords",
            engine.scope_keywords(),
            ledger
                .claims
                .iter()
                .filter(|claim| claim.conditions.is_empty())
                .filter_map(language_subject::scope_keyword)
                .collect(),
        ),
        compare(
            "scope_links",
            engine.names(SubjectKind::ScopeLink),
            config_names(SubjectKind::ScopeLink),
        ),
        compare(
            "localisation_commands",
            engine.names(SubjectKind::LocalizationCommand),
            config_names(SubjectKind::LocalizationCommand),
        ),
        compare(
            "localisation_links",
            engine.names(SubjectKind::LocalizationLink),
            config_names(SubjectKind::LocalizationLink),
        ),
        compare(
            "on_actions",
            engine.names(SubjectKind::OnAction),
            config_names(SubjectKind::OnAction),
        ),
        compare(
            "game_rules",
            engine.names(SubjectKind::GameRule),
            config_names(SubjectKind::GameRule),
        ),
        compare(
            "defines",
            engine.names(SubjectKind::Define),
            config_names(SubjectKind::Define),
        ),
    ]
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
