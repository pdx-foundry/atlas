//! Loader rules: how each registry's loader selects its files, treats a repeated key and names
//! its definitions.

use super::{
    EvidenceLink, Snapshot, Subject, SubjectKind, evidence, field_id, gap, registry_id, rule,
};
use crate::extraction::Extraction;
use pdx_native::{
    Answer, DefinitionForm, DefinitionStatement, GapSubject, GrammarProperty, LoaderRules,
};
use serde_json::{Value, json};

/// Property families that loader-rule assembly can establish.
pub(super) const PROPERTIES: &[&str] = &[
    "extension",
    "recursion",
    "duplicate_rule",
    "definition_names",
];

/// The rules that Native states once for the build and attaches to every answer.
const BUILD_WIDE: &[&str] = &["file_order", "mod_over_vanilla"];

/// Assemble the loader rules of every requested registry, then the build-wide limits once.
pub(super) fn assemble(snapshot: &mut Snapshot, extraction: &Extraction) -> Result<(), String> {
    for (registry, result) in &extraction.loader_rules {
        match result {
            Ok(answer) => assemble_registry(snapshot, registry, answer)?,
            Err(error) => {
                for property in PROPERTIES {
                    gap(
                        snapshot,
                        &registry_id(registry),
                        property,
                        format!(
                            "Native loader-rule question failed: {}",
                            super::failure::error_reason(error)
                        ),
                        None,
                        Vec::new(),
                        Vec::new(),
                    );
                }
            }
        }
    }

    let first_answer = extraction
        .loader_rules
        .iter()
        .find_map(|(registry, result)| Some((registry, result.as_ref().ok()?)));

    if let Some((registry, answer)) = first_answer {
        assemble_build_wide(snapshot, registry, answer)?;
    }

    Ok(())
}

fn assemble_registry(
    snapshot: &mut Snapshot,
    registry: &str,
    answer: &Answer<LoaderRules>,
) -> Result<(), String> {
    let rules = &answer.value;
    let extension = map(&rules.extension, |suffix| json!({ "suffix": suffix }));
    let recursion = map(&rules.recursion, |recursion| json!(recursion));
    let duplicate_rule = map(&rules.duplicate_rule, |duplicate_rule| {
        json!(duplicate_rule)
    });
    let root_forms = root_forms(&rules.definition_names);

    for (property, value) in [
        ("extension", extension),
        ("recursion", recursion),
        ("duplicate_rule", duplicate_rule),
        ("definition_names", root_forms),
    ] {
        let link = link(snapshot, registry, answer, property)?;

        assemble_property(snapshot, registry, property, value, link);
    }

    let nested_forms = match &rules.definition_names {
        GrammarProperty::Known(forms) | GrammarProperty::Partial(forms) => forms.as_slice(),
        GrammarProperty::Unresolved => &[],
    };

    assemble_nested_forms(snapshot, registry, answer, nested_forms)
}

/// A rule for an established property; for a partial one, also a gap that publishes what is
/// missing without blocking the stated part; for an unresolved one, a gap.
fn assemble_property(
    snapshot: &mut Snapshot,
    registry: &str,
    property: &str,
    value: GrammarProperty<Value>,
    link: EvidenceLink,
) {
    let id = registry_id(registry);
    let native_gaps = link.native_gaps.clone();

    match value {
        GrammarProperty::Known(value) => rule(snapshot, &id, property, value, link),
        GrammarProperty::Partial(value) => {
            gap(
                snapshot,
                &id,
                &format!("{property}.partial"),
                format!("Native states only part of this registry's {property}"),
                None,
                native_gaps,
                vec![link.clone()],
            );
            rule(snapshot, &id, property, value, link);
        }
        GrammarProperty::Unresolved => gap(
            snapshot,
            &id,
            property,
            format!("Native did not establish this registry's {property}"),
            None,
            native_gaps,
            vec![link],
        ),
    }
}

/// A rule on each field whose nested definitions a form names. A form at a field path that
/// Native's field answer does not list stays a gap on the registry.
fn assemble_nested_forms(
    snapshot: &mut Snapshot,
    registry: &str,
    answer: &Answer<LoaderRules>,
    forms: &[DefinitionForm],
) -> Result<(), String> {
    let mut unplaced = Vec::new();

    for form in forms {
        let DefinitionStatement::Nested { field_path } = &form.statement else {
            continue;
        };
        let path = field_path.join("/");
        let id = field_id(registry, &path);

        if !snapshot.subjects.iter().any(|subject| subject.id == id) {
            unplaced.push(path);
            continue;
        }

        let link = link(snapshot, registry, answer, "definition_names")?;

        rule(snapshot, &id, "definition_names", json!(form), link);
    }

    if !unplaced.is_empty() {
        let link = link(snapshot, registry, answer, "definition_names")?;

        gap(
            snapshot,
            &registry_id(registry),
            "definition_names.nested",
            format!(
                "Native names definitions nested at {}, but no field subject has that path",
                unplaced.join(", ")
            ),
            None,
            Vec::new(),
            vec![link],
        );
    }

    Ok(())
}

/// One gap for each build-wide rule, on one inventory subject: Native states these rules once,
/// with no fixture that verifies them.
fn assemble_build_wide(
    snapshot: &mut Snapshot,
    registry: &str,
    answer: &Answer<LoaderRules>,
) -> Result<(), String> {
    let id = SubjectKind::Inventory.id("loader_rules");

    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: SubjectKind::Inventory,
        registry: None,
        field: None,
        conditional: None,
        name: Some("loader_rules".into()),
    });

    for property in BUILD_WIDE {
        let link = link(snapshot, registry, answer, property)?;
        let reason = link
            .native_gaps
            .iter()
            .map(|gap| gap.detail.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let reason = if reason.is_empty() {
            format!("Native does not state the build's {property}")
        } else {
            reason
        };

        gap(
            snapshot,
            &id,
            property,
            reason,
            None,
            link.native_gaps.clone(),
            vec![link],
        );
    }

    Ok(())
}

/// The evidence link to one property of a registry's answer, with that property's gaps.
fn link(
    snapshot: &mut Snapshot,
    registry: &str,
    answer: &Answer<LoaderRules>,
    property: &str,
) -> Result<EvidenceLink, String> {
    evidence(
        snapshot,
        &format!("loader_rules/{registry}"),
        answer,
        format!("answers.loader_rules.{registry}:{property}"),
        Some(GapSubject::AnswerItem {
            name: property.into(),
        }),
    )
}

fn map<T>(property: &GrammarProperty<T>, render: impl Fn(&T) -> Value) -> GrammarProperty<Value> {
    match property {
        GrammarProperty::Known(value) => GrammarProperty::Known(render(value)),
        GrammarProperty::Partial(value) => GrammarProperty::Partial(render(value)),
        GrammarProperty::Unresolved => GrammarProperty::Unresolved,
    }
}

/// The forms of root statements. A known empty list means that the loader names no root
/// definition; a partial answer that states no root form leaves the root forms unresolved.
fn root_forms(names: &GrammarProperty<Vec<DefinitionForm>>) -> GrammarProperty<Value> {
    let roots = |forms: &[DefinitionForm]| -> Vec<Value> {
        forms
            .iter()
            .filter(|form| !matches!(form.statement, DefinitionStatement::Nested { .. }))
            .map(|form| json!(form))
            .collect()
    };

    match names {
        GrammarProperty::Known(forms) => GrammarProperty::Known(json!(roots(forms))),
        GrammarProperty::Partial(forms) if roots(forms).is_empty() => GrammarProperty::Unresolved,
        GrammarProperty::Partial(forms) => GrammarProperty::Partial(json!(roots(forms))),
        GrammarProperty::Unresolved => GrammarProperty::Unresolved,
    }
}
