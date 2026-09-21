//! Atlas's questions and their mapping to the tradition config rules.

use serde::Serialize;

pub(super) const TRADITIONS: &str = "common/traditions";
pub(super) const CATEGORIES: &str = "common/tradition_categories";

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) enum Promise {
    Structures,
    Bonuses,
    References,
    Conditions,
    Modifiers,
    Effects,
    SwapsAndInheritance,
    Weights,
    LocalisationAndTooltips,
    Icons,
    TreeTemplates,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Check {
    Registry(&'static str),
    Items(&'static str),
    Field(&'static str, &'static str),
    Reader(&'static str, &'static str),
    SharedReader,
    Storage(&'static str, &'static str, &'static str),
    Diagnostic(&'static str, &'static str),
    DiagnosticCoverage(&'static str),
    Runtime,
    CategoryRead(&'static str),
    RegistrationEntries,
    TreeTemplateRegistry,
    Missing(&'static str),
}

#[derive(Debug, Clone)]
pub(super) struct Question {
    pub id: String,
    pub rule: &'static str,
    pub promise: Promise,
    pub check: Check,
    pub owner: &'static str,
}

struct FieldRule {
    registry: &'static str,
    name: &'static str,
    rule: &'static str,
    promise: Promise,
    owner: &'static str,
}

// Native discovers root fields. The rule path records the matching CWT rule, or an engine-only
// field that the current CWT file does not describe. Nested CWT questions are listed below.
const ROOT_FIELDS: &[FieldRule] = &[
    FieldRule {
        registry: TRADITIONS,
        name: "unlocks_agenda",
        rule: "tradition.unlocks_agenda",
        promise: Promise::References,
        owner: "SDK-543",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "modifier",
        rule: "tradition.modifier",
        promise: Promise::Modifiers,
        owner: "SDK-542",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "triggered_modifier",
        rule: "tradition.triggered_modifier",
        promise: Promise::Modifiers,
        owner: "SDK-541",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "possible",
        rule: "tradition.possible",
        promise: Promise::Conditions,
        owner: "SDK-542",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "potential",
        rule: "engine-only:tradition.potential",
        promise: Promise::Conditions,
        owner: "SDK-541",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "on_enabled",
        rule: "tradition.on_enabled",
        promise: Promise::Effects,
        owner: "SDK-542",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "on_disabled",
        rule: "engine-only:tradition.on_disabled",
        promise: Promise::Effects,
        owner: "SDK-541",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "custom_tooltip",
        rule: "tradition.custom_tooltip",
        promise: Promise::LocalisationAndTooltips,
        owner: "SDK-546",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "custom_tooltip_with_modifiers",
        rule: "tradition.custom_tooltip_with_modifiers",
        promise: Promise::LocalisationAndTooltips,
        owner: "SDK-546",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "tradition_swap",
        rule: "tradition.tradition_swap",
        promise: Promise::SwapsAndInheritance,
        owner: "SDK-541",
    },
    FieldRule {
        registry: TRADITIONS,
        name: "ai_weight",
        rule: "tradition.ai_weight",
        promise: Promise::Weights,
        owner: "SDK-545",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "desc",
        rule: "tradition_category.desc",
        promise: Promise::LocalisationAndTooltips,
        owner: "SDK-541",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "tree_template",
        rule: "tradition_category.tree_template",
        promise: Promise::TreeTemplates,
        owner: "SDK-543",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "adoption_bonus",
        rule: "tradition_category.adoption_bonus",
        promise: Promise::Bonuses,
        owner: "SDK-541",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "finish_bonus",
        rule: "tradition_category.finish_bonus",
        promise: Promise::Bonuses,
        owner: "SDK-541",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "traditions",
        rule: "tradition_category.traditions",
        promise: Promise::References,
        owner: "SDK-541",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "potential",
        rule: "tradition_category.potential",
        promise: Promise::Conditions,
        owner: "SDK-542",
    },
    FieldRule {
        registry: CATEGORIES,
        name: "ai_weight",
        rule: "tradition_category.ai_weight",
        promise: Promise::Weights,
        owner: "SDK-545",
    },
];

pub(super) fn questions() -> Vec<Question> {
    let mut result = vec![
        question(
            "tradition.registry",
            "type[tradition].path",
            Promise::Structures,
            Check::Registry(TRADITIONS),
            "SDK-529",
        ),
        question(
            "category.registry",
            "type[tradition_category].path",
            Promise::Structures,
            Check::Registry(CATEGORIES),
            "SDK-529",
        ),
        question(
            "tradition.items",
            "type[tradition].path",
            Promise::Structures,
            Check::Items(TRADITIONS),
            "SDK-529",
        ),
        question(
            "category.items",
            "type[tradition_category].path",
            Promise::Structures,
            Check::Items(CATEGORIES),
            "SDK-529",
        ),
    ];
    for field in ROOT_FIELDS {
        let family = if field.registry == TRADITIONS {
            "tradition"
        } else {
            "category"
        };
        result.push(question(
            format!("{family}.{}.field", field.name),
            field.rule,
            field.promise,
            Check::Field(field.registry, field.name),
            field.owner,
        ));
        result.push(question(
            format!("{family}.{}.reader", field.name),
            field.rule,
            field.promise,
            Check::Reader(field.registry, field.name),
            field.owner,
        ));
        if field.registry == CATEGORIES
            || !matches!(
                field.name,
                "unlocks_agenda" | "custom_tooltip" | "custom_tooltip_with_modifiers"
            )
        {
            let (session, definition) = if field.registry == TRADITIONS {
                ("tradition_outcomes", "atlas_valid")
            } else {
                ("category_outcomes", "atlas_category")
            };
            result.push(question(
                format!("{family}.{}.storage.valid", field.name),
                field.rule,
                field.promise,
                Check::Storage(session, definition, field.name),
                "SDK-541",
            ));
        }
    }
    result.extend([
        question(
            "tradition.tooltip.shared_reader",
            "tradition.custom_tooltip",
            Promise::LocalisationAndTooltips,
            Check::SharedReader,
            "SDK-531",
        ),
        question(
            "tradition.agenda.storage.valid",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Storage("tradition_outcomes", "atlas_valid", "unlocks_agenda"),
            "SDK-541",
        ),
        question(
            "tradition.tooltip.storage.valid",
            "tradition.custom_tooltip",
            Promise::LocalisationAndTooltips,
            Check::Storage("tradition_outcomes", "atlas_valid", "custom_tooltip"),
            "SDK-541",
        ),
        question(
            "tradition.tooltip_modifiers.storage.valid",
            "tradition.custom_tooltip_with_modifiers",
            Promise::LocalisationAndTooltips,
            Check::Storage(
                "tradition_outcomes",
                "atlas_valid",
                "custom_tooltip_with_modifiers",
            ),
            "SDK-541",
        ),
        question(
            "tradition.agenda.storage.omitted",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Storage("tradition_outcomes", "atlas_omitted", "unlocks_agenda"),
            "SDK-541",
        ),
        question(
            "tradition.agenda.storage.repeated",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Storage("tradition_outcomes", "atlas_repeated", "unlocks_agenda"),
            "SDK-541",
        ),
        question(
            "tradition.agenda.storage.malformed",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Storage("tradition_outcomes", "atlas_malformed", "unlocks_agenda"),
            "SDK-541",
        ),
        question(
            "tradition.agenda.storage.unknown_field",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Storage("tradition_outcomes", "atlas_unknown", "unlocks_agenda"),
            "SDK-541",
        ),
        question(
            "tradition.parser.coverage",
            "tradition.*",
            Promise::Structures,
            Check::DiagnosticCoverage("tradition_outcomes"),
            "SDK-541",
        ),
        question(
            "tradition.parser.malformed",
            "tradition.unlocks_agenda",
            Promise::Structures,
            Check::Diagnostic("Malformed token", "broken\""),
            "SDK-541",
        ),
        question(
            "tradition.parser.unknown_field",
            "tradition.*",
            Promise::Structures,
            Check::Diagnostic("Unexpected token", "this_is_an_unknown_field"),
            "SDK-541",
        ),
        question(
            "tradition.agenda.runtime",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Runtime,
            "SDK-541",
        ),
        question(
            "category.parser.coverage",
            "tradition_category.*",
            Promise::Structures,
            Check::DiagnosticCoverage("category_outcomes"),
            "SDK-541",
        ),
        question(
            "category.tree_template.read",
            "tradition_category.tree_template",
            Promise::TreeTemplates,
            Check::CategoryRead("tree_template"),
            "SDK-532",
        ),
        question(
            "category.traditions.read",
            "tradition_category.traditions",
            Promise::References,
            Check::CategoryRead("traditions"),
            "SDK-532",
        ),
        question(
            "category.registration_entries",
            "type[tradition_category].path",
            Promise::Effects,
            Check::RegistrationEntries,
            "SDK-532",
        ),
        question(
            "tree_template.registry",
            "tradition_category.tree_template",
            Promise::TreeTemplates,
            Check::TreeTemplateRegistry,
            "SDK-551",
        ),
    ]);
    result.extend([
        question(
            "tradition.modifier.clause",
            "tradition.modifier.*",
            Promise::Modifiers,
            Check::Missing("Native does not interpret modifier clauses"),
            "SDK-536",
        ),
        question(
            "tradition.triggered_modifier.clause",
            "tradition.triggered_modifier.*",
            Promise::Modifiers,
            Check::Missing("Native does not interpret conditional modifier clauses"),
            "SDK-542",
        ),
        question(
            "tradition.possible.scope",
            "tradition.possible.*",
            Promise::Conditions,
            Check::Missing("Native does not establish trigger scope or predicates"),
            "SDK-549",
        ),
        question(
            "tradition.on_enabled.effect",
            "tradition.on_enabled.*",
            Promise::Effects,
            Check::Missing("Native does not establish effect declarations or behavior"),
            "SDK-535",
        ),
        question(
            "tradition.swap.inheritance",
            "tradition.tradition_swap.inherit_*",
            Promise::SwapsAndInheritance,
            Check::Missing("Native does not establish swap inheritance"),
            "SDK-542",
        ),
        question(
            "tradition.swap.weight",
            "tradition.tradition_swap.weight",
            Promise::Weights,
            Check::Missing("Native does not establish weight rules"),
            "SDK-545",
        ),
        question(
            "tradition.agenda.reference",
            "tradition.unlocks_agenda",
            Promise::References,
            Check::Missing("String storage does not establish agenda resolution"),
            "SDK-543",
        ),
        question(
            "tradition.ai_weight.rule",
            "tradition.ai_weight.*",
            Promise::Weights,
            Check::Missing("Native does not establish weight rules"),
            "SDK-545",
        ),
        question(
            "tradition.localisation",
            "type[tradition].localisation",
            Promise::LocalisationAndTooltips,
            Check::Missing("Native does not establish localisation lookup"),
            "SDK-546",
        ),
        question(
            "tradition.icon",
            "type[tradition].images.icon",
            Promise::Icons,
            Check::Missing("Native does not establish inferred icon paths"),
            "SDK-546",
        ),
        question(
            "category.bonus.references",
            "tradition_category.adoption_bonus|finish_bonus",
            Promise::Bonuses,
            Check::Missing("Native does not resolve bonus tradition references"),
            "SDK-543",
        ),
        question(
            "category.traditions.references",
            "tradition_category.traditions.*",
            Promise::References,
            Check::Missing("Native does not resolve member tradition references"),
            "SDK-543",
        ),
        question(
            "category.desc.clause",
            "tradition_category.desc.*",
            Promise::LocalisationAndTooltips,
            Check::Missing("Native does not establish triggered description clauses"),
            "SDK-542",
        ),
        question(
            "category.localisation",
            "type[tradition_category].localisation",
            Promise::LocalisationAndTooltips,
            Check::Missing("Native does not establish localisation lookup"),
            "SDK-546",
        ),
        question(
            "category.potential.scope",
            "tradition_category.potential.*",
            Promise::Conditions,
            Check::Missing("Native does not establish trigger scope or predicates"),
            "SDK-549",
        ),
        question(
            "category.ai_weight.rule",
            "tradition_category.ai_weight.*",
            Promise::Weights,
            Check::Missing("Native does not establish weight rules"),
            "SDK-545",
        ),
        question(
            "category.tree_template.reference",
            "tradition_category.tree_template",
            Promise::TreeTemplates,
            Check::Missing("String storage does not establish tree-template resolution"),
            "SDK-543",
        ),
    ]);
    result
}

fn question(
    id: impl Into<String>,
    rule: &'static str,
    promise: Promise,
    check: Check,
    owner: &'static str,
) -> Question {
    Question {
        id: id.into(),
        rule,
        promise,
        check,
        owner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_requested_storage_outcome_has_one_atlas_question() {
        let asked: BTreeSet<_> = [
            ("tradition_outcomes", crate::frozen::tradition_fixture()),
            ("category_outcomes", crate::frozen::category_fixture()),
        ]
        .into_iter()
        .flat_map(|(session, fixture)| {
            fixture
                .field_questions
                .into_iter()
                .map(move |field| (session, field.definition, field.field))
        })
        .collect();
        let rows = questions();
        let covered: BTreeSet<_> = rows
            .iter()
            .filter_map(|row| match row.check {
                Check::Storage(session, definition, field) => {
                    Some((session, definition.to_owned(), field.to_owned()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(asked, covered);
        assert_eq!(
            rows.len(),
            rows.iter()
                .map(|row| &row.id)
                .collect::<BTreeSet<_>>()
                .len()
        );
    }
}
