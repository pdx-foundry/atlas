//! Language declaration rules: commands, modifiers, scopes, links, localization, callbacks and
//! defines.
//!
//! A named item gives an existence rule even when Native's answer is partial: a partial answer
//! keeps each part that it establishes, and Atlas never claims that the list is complete. Each
//! typed value that Native could not follow becomes a gap on its property, and each Native gap
//! becomes a `native.{question}.{kind}` gap on the item that it names or on the question.

use super::{
    EvidenceLink, Snapshot, Subject, SubjectKind, conditional_rule, evidence, gap, registry_id,
    rule,
};
use crate::extraction::{Extraction, LoadedModifierSession};
use pdx_native::{
    Answer, ContextScopes, Declaration, DeclaredScopes, DeclaredTags, Define, DefineValueType,
    Disposal, EntryContext, EntryScope, Error, GameRule, Gap as NativeGap, GapKind, GapSubject,
    GenerationCondition, LinkData, LoadedContent, LoadedModifiers, LocalizationContextId,
    LocalizationContextReference, LocalizationDeclarations, LocalizationOutput, ModifierCategory,
    ModifierDeclaration, ModifierFamily, NamePart, OnAction, OutputScope, RuleKind, ScopeId,
    ScopeInventory, ScopeLink, ScopeReference,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Property families that language assembly can establish.
pub(super) const PROPERTIES: &[&str] = &[
    "alternatives",
    "category_tags",
    "contexts",
    "data",
    "declared_scopes",
    "documentation",
    "entry_scopes",
    "existence",
    "generation",
    "groups",
    "input_scopes",
    "keywords",
    "kind",
    "loaded_summary",
    "members",
    "name_limit",
    "name_template",
    "output_scope",
    "registry",
    "scopes",
    "value_type",
];

const ARGUMENT_GRAMMARS: &str = "SDK-548";
const SCOPE_CONTEXT: &str = "SDK-549";
const MODIFIER_APPLICATION: &str = "SDK-547";
const REFERENCES: &str = "SDK-543";
const CALLBACK_CONTEXTS: &str = "SDK-496";

pub(super) fn assemble(snapshot: &mut Snapshot, extraction: &Extraction) -> Result<(), String> {
    let language = &extraction.language;
    let mut builder = Builder {
        snapshot,
        subjects: BTreeSet::new(),
        scopes: match &language.scopes {
            Ok(answer) => scope_names(&answer.value)?,
            Err(_) => BTreeMap::new(),
        },
    };

    builder.declarations(SubjectKind::Effect, "effect", &language.effects)?;
    builder.declarations(SubjectKind::Trigger, "trigger", &language.triggers)?;
    builder.modifiers(&language.modifiers)?;
    builder.modifier_categories(&language.modifier_categories)?;
    for (registry, answer) in &language.modifier_families {
        builder.modifier_families(registry, answer)?;
    }
    builder.scope_inventory(&language.scopes)?;
    builder.scope_links(&language.scope_links)?;
    builder.localization(&language.localization)?;
    builder.on_actions(&language.on_actions)?;
    builder.game_rules(&language.game_rules)?;
    builder.defines(&language.defines)?;
    builder.loaded_modifiers(&extraction.loaded_modifiers, &extraction.fields)
}

/// Where a Native gap belongs.
enum Target {
    /// A language subject, created when absent.
    Language(SubjectKind, String),
    /// A subject that registry assembly made.
    Existing(String),
}

fn item_target(subject: &GapSubject, kind: SubjectKind) -> Option<Target> {
    match subject {
        GapSubject::Item { name } => Some(Target::Language(kind, name.clone())),
        _ => None,
    }
}

struct Builder<'a> {
    snapshot: &'a mut Snapshot,
    subjects: BTreeSet<String>,
    /// Subject name of each scope type in Native's scope inventory.
    scopes: BTreeMap<ScopeId, String>,
}

impl Builder<'_> {
    fn subject(&mut self, kind: SubjectKind, name: &str) -> String {
        let id = kind.id(name);

        if self.subjects.insert(id.clone()) {
            self.snapshot.subjects.push(Subject {
                id: id.clone(),
                kind,
                registry: None,
                field: None,
                conditional: None,
                name: Some(name.into()),
            });
        }

        id
    }

    fn inventory(&mut self, question: &str) -> String {
        self.subject(SubjectKind::Inventory, question)
    }

    /// The answer of a question, or a gap on `subject` when the question failed.
    fn answered<'r, T>(
        &mut self,
        subject: &str,
        property: &str,
        result: &'r Result<Answer<T>, Error>,
    ) -> Option<&'r Answer<T>> {
        match result {
            Ok(answer) => Some(answer),
            Err(error) => {
                self.gap(
                    subject,
                    property,
                    format!("Native question failed: {error}"),
                    None,
                    Vec::new(),
                );
                None
            }
        }
    }

    fn evidence<T>(
        &mut self,
        key: &str,
        answer: &Answer<T>,
        location_suffix: &str,
        subject: Option<GapSubject>,
    ) -> Result<EvidenceLink, String> {
        evidence(
            self.snapshot,
            key,
            answer,
            format!("{key}:{location_suffix}"),
            subject,
        )
    }

    fn rule(&mut self, subject: &str, property: &str, answer: Value, link: &EvidenceLink) {
        rule(self.snapshot, subject, property, answer, link.clone());
    }

    fn gap(
        &mut self,
        subject: &str,
        property: &str,
        reason: impl Into<String>,
        owner: Option<&str>,
        evidence: Vec<EvidenceLink>,
    ) {
        gap(
            self.snapshot,
            subject,
            property,
            reason,
            owner,
            Vec::new(),
            evidence,
        );
    }

    /// Record each Native gap of `answer` on the subject that `target` names, or on `fallback`.
    /// Gaps of one kind on one subject form one snapshot gap.
    fn native_gaps<T>(
        &mut self,
        question: &str,
        key: &str,
        answer: &Answer<T>,
        fallback: &str,
        target: impl Fn(&GapSubject) -> Option<Target>,
    ) -> Result<(), String> {
        let mut grouped: BTreeMap<(String, &str), Vec<NativeGap>> = BTreeMap::new();

        for native_gap in &answer.gaps {
            let subject = match native_gap.subject.as_ref().and_then(&target) {
                Some(Target::Language(kind, name)) => self.subject(kind, &name),
                Some(Target::Existing(id)) => id,
                None => fallback.to_owned(),
            };

            grouped
                .entry((subject, gap_kind(native_gap.kind)))
                .or_default()
                .push(native_gap.clone());
        }

        for ((subject, kind), native_gaps) in grouped {
            let mut details = Vec::new();

            for native_gap in &native_gaps {
                if !details.contains(&native_gap.detail) {
                    details.push(native_gap.detail.clone());
                }
            }

            let link = evidence(self.snapshot, key, answer, format!("{key}:gaps"), None)?;

            gap(
                self.snapshot,
                &subject,
                &format!("native.{question}.{kind}"),
                details.join("; "),
                None,
                native_gaps,
                vec![link],
            );
        }

        Ok(())
    }

    /// Scope subject identities for `references`, or `None` when one is not in the inventory.
    fn scope_ids(&self, references: &[ScopeReference]) -> Option<Vec<String>> {
        references
            .iter()
            .map(|reference| {
                self.scopes
                    .get(&reference.id)
                    .map(|name| SubjectKind::Scope.id(name))
            })
            .collect()
    }

    fn scope_set(
        &mut self,
        subject: &str,
        property: &str,
        scopes: &DeclaredScopes,
        link: &EvidenceLink,
    ) {
        match scopes {
            DeclaredScopes::Any => self.rule(subject, property, json!("any"), link),
            DeclaredScopes::Listed(references) => match self.scope_ids(references) {
                Some(ids) => self.rule(subject, property, json!(ids), link),
                None => self.unknown_scope(subject, property, link),
            },
            DeclaredScopes::Unresolved => self.gap(
                subject,
                property,
                "Native did not follow the declaration to its scope set",
                None,
                vec![link.clone()],
            ),
        }
    }

    fn unknown_scope(&mut self, subject: &str, property: &str, link: &EvidenceLink) {
        self.gap(
            subject,
            property,
            "A declared scope type is not in Native's scope inventory answer",
            None,
            vec![link.clone()],
        );
    }

    fn category_tags(&mut self, subject: &str, tags: &DeclaredTags, link: &EvidenceLink) {
        match tags {
            DeclaredTags::Listed(tags) => self.rule(subject, "category_tags", json!(tags), link),
            DeclaredTags::Unresolved => self.gap(
                subject,
                "category_tags",
                "Native did not follow the declaration to its category tags",
                None,
                vec![link.clone()],
            ),
        }
    }

    fn declarations(
        &mut self,
        kind: SubjectKind,
        question: &str,
        result: &Result<Answer<Vec<Declaration>>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory(&format!("{question}s"));
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };
        let key = format!("declarations/{question}");

        for declaration in &answer.value {
            let id = self.subject(kind, &declaration.name);
            let link = self.evidence(
                &key,
                answer,
                &declaration.name,
                Some(GapSubject::Item {
                    name: declaration.name.clone(),
                }),
            )?;

            self.rule(&id, "existence", json!(true), &link);
            self.scope_set(&id, "declared_scopes", &declaration.scopes, &link);

            if declaration.description.is_empty() && declaration.usage.is_empty() {
                self.gap(
                    &id,
                    "documentation",
                    "Native returned no documentation text for this command",
                    None,
                    vec![link.clone()],
                );
            } else {
                let documentation = json!({
                    "description": declaration.description,
                    "usage": declaration.usage,
                    "text_origin": "engine",
                });

                self.rule(&id, "documentation", documentation, &link);
            }

            self.gap(
                &id,
                "arguments",
                "The command's argument grammar, including target arguments, is not established",
                Some(ARGUMENT_GRAMMARS),
                vec![link.clone()],
            );
            self.gap(
                &id,
                "scope_context",
                "The scope that the command's block enters is not established",
                Some(SCOPE_CONTEXT),
                vec![link],
            );
        }

        self.native_gaps(question, &key, answer, &inventory, |subject| {
            item_target(subject, kind)
        })
    }

    fn modifiers(
        &mut self,
        result: &Result<Answer<Vec<ModifierDeclaration>>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory("modifiers");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for modifier in &answer.value {
            let id = self.subject(SubjectKind::Modifier, &modifier.name);
            let link = self.evidence(
                "modifiers",
                answer,
                &modifier.name,
                Some(GapSubject::Item {
                    name: modifier.name.clone(),
                }),
            )?;

            self.rule(&id, "existence", json!(true), &link);
            self.category_tags(&id, &modifier.category_tags, &link);
            self.gap(
                &id,
                "application",
                "Category tags are intended-use tags; where the modifier takes effect is not established",
                Some(MODIFIER_APPLICATION),
                vec![link],
            );
        }

        self.native_gaps("modifiers", "modifiers", answer, &inventory, |subject| {
            item_target(subject, SubjectKind::Modifier)
        })
    }

    fn modifier_categories(
        &mut self,
        result: &Result<Answer<Vec<ModifierCategory>>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory("modifier_categories");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for category in &answer.value {
            let id = self.subject(SubjectKind::ModifierCategory, &category.name);
            let link = self.evidence(
                "modifier_categories",
                answer,
                &category.name,
                Some(GapSubject::Item {
                    name: category.name.clone(),
                }),
            )?;

            self.rule(&id, "existence", json!(true), &link);
            self.gap(
                &id,
                "supported_scopes",
                "A category is an intended-use tag; the scopes where its modifiers take effect are not established",
                Some(MODIFIER_APPLICATION),
                vec![link],
            );
        }

        self.native_gaps(
            "modifier_categories",
            "modifier_categories",
            answer,
            &inventory,
            |subject| item_target(subject, SubjectKind::ModifierCategory),
        )
    }

    fn modifier_families(
        &mut self,
        registry: &str,
        result: &Result<Answer<Vec<ModifierFamily>>, Error>,
    ) -> Result<(), String> {
        let registry_subject = registry_id(registry);
        let Some(answer) = self.answered(&registry_subject, "modifier_families", result) else {
            return Ok(());
        };
        let key = format!("modifier_families/{registry}");

        for family in &answer.value {
            let template = name_template(&family.name);
            let name = format!("{registry}/{}", template.text);
            let id = self.subject(SubjectKind::ModifierFamily, &name);
            let link = self.evidence(&key, answer, &template.text, None)?;

            self.rule(&id, "registry", json!(registry), &link);

            match template.parts {
                Some(parts) => self.rule(&id, "name_template", parts, &link),
                None => self.gap(
                    &id,
                    "name_template",
                    "Native returned a name part that Atlas does not interpret",
                    None,
                    vec![link.clone()],
                ),
            }

            self.category_tags(&id, &family.category_tags, &link);

            if let Some(limit) = family.name_limit {
                self.rule(&id, "name_limit", json!(limit), &link);
            }

            match family.condition {
                GenerationCondition::Always => {
                    self.rule(&id, "generation", json!("every_item"), &link)
                }
                _ => self.gap(
                    &id,
                    "generation",
                    "Native does not establish that every item of the registry generates this family",
                    None,
                    vec![link],
                ),
            }
        }

        self.native_gaps(
            "modifier_families",
            &key,
            answer,
            &registry_subject,
            |subject| match subject {
                GapSubject::Registry { name } => Some(Target::Existing(registry_id(name))),
                _ => None,
            },
        )
    }

    fn scope_inventory(
        &mut self,
        result: &Result<Answer<ScopeInventory>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory("scopes");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for scope in &answer.value.types {
            let name = self.scopes[&scope.id].clone();
            let id = self.subject(SubjectKind::Scope, &name);
            let link = self.evidence(
                "scopes",
                answer,
                &name,
                Some(GapSubject::ScopeType {
                    id: scope.id.clone(),
                    name: scope.name.clone(),
                }),
            )?;
            let groups: Vec<_> = answer
                .value
                .groups
                .iter()
                .filter(|group| group.scopes.iter().any(|member| member.id == scope.id))
                .map(|group| group.keyword.as_str())
                .collect();

            self.rule(&id, "existence", json!(true), &link);
            self.rule(&id, "keywords", json!(scope.keywords), &link);
            self.rule(&id, "groups", json!(groups), &link);
        }

        for group in &answer.value.groups {
            let id = self.subject(SubjectKind::ScopeGroup, &group.keyword);
            let link = self.evidence(
                "scopes",
                answer,
                &group.keyword,
                Some(GapSubject::Item {
                    name: group.keyword.clone(),
                }),
            )?;

            self.rule(&id, "existence", json!(true), &link);

            match self.scope_ids(&group.scopes) {
                Some(members) => self.rule(&id, "members", json!(members), &link),
                None => self.unknown_scope(&id, "members", &link),
            }
        }

        self.native_gaps("scopes", "scopes", answer, &inventory, |subject| {
            item_target(subject, SubjectKind::ScopeGroup)
        })
    }

    fn scope_links(
        &mut self,
        result: &Result<Answer<Vec<ScopeLink>>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory("scope_links");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for link_declaration in &answer.value {
            let id = self.subject(SubjectKind::ScopeLink, &link_declaration.name);
            let link = self.evidence(
                "scope_links",
                answer,
                &link_declaration.name,
                Some(GapSubject::Item {
                    name: link_declaration.name.clone(),
                }),
            )?;

            self.rule(&id, "existence", json!(true), &link);
            self.scope_set(&id, "input_scopes", &link_declaration.input_scopes, &link);

            match &link_declaration.output_scope {
                OutputScope::Listed(references) => match self.scope_ids(references) {
                    Some(ids) => self.rule(&id, "output_scope", json!(ids), &link),
                    None => self.unknown_scope(&id, "output_scope", &link),
                },
                OutputScope::Various => self.rule(&id, "output_scope", json!("various"), &link),
                OutputScope::Unresolved => self.gap(
                    &id,
                    "output_scope",
                    "Native did not follow the link to its output scope",
                    None,
                    vec![link.clone()],
                ),
            }

            match &link_declaration.data {
                LinkData::None => self.rule(&id, "data", json!({"takes_data": false}), &link),
                LinkData::Prefix(prefix) => {
                    self.rule(
                        &id,
                        "data",
                        json!({"takes_data": true, "prefix": prefix}),
                        &link,
                    );
                    self.gap(
                        &id,
                        "data_source",
                        "What the data after the link's prefix refers to is not established",
                        Some(REFERENCES),
                        vec![link],
                    );
                }
                _ => self.gap(
                    &id,
                    "data",
                    "Native returned a link data form that Atlas does not interpret",
                    None,
                    vec![link],
                ),
            }
        }

        self.native_gaps(
            "scope_links",
            "scope_links",
            answer,
            &inventory,
            |subject| item_target(subject, SubjectKind::ScopeLink),
        )
    }

    fn localization(
        &mut self,
        result: &Result<Answer<LocalizationDeclarations>, Error>,
    ) -> Result<(), String> {
        let inventory = self.inventory("localization");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };
        let key = "localization_declarations";
        let declarations = &answer.value;
        let contexts = context_names(declarations)?;
        // Native drops the rows of a context whose table it cannot read, so no command or link
        // lists every context that declares it.
        let rows_complete = !answer
            .gaps
            .iter()
            .any(|native_gap| native_gap.kind == GapKind::UnreadableInput);
        let mut selecting: BTreeMap<&LocalizationContextId, Option<Vec<String>>> = BTreeMap::new();

        for context in &declarations.contexts {
            let name = &contexts[&context.id];
            let id = self.subject(SubjectKind::LocalizationContext, name);
            let link = self.evidence(
                key,
                answer,
                name,
                Some(GapSubject::LocalizationContext {
                    id: context.id.clone(),
                    name: context.name.clone(),
                }),
            )?;
            let scopes = match &context.scopes {
                ContextScopes::Joined(references) => self.scope_ids(references),
                ContextScopes::Missing => Some(Vec::new()),
                ContextScopes::Partial(_) => None,
            };

            self.rule(&id, "existence", json!(true), &link);

            match &scopes {
                Some(scopes) => self.rule(&id, "scopes", json!(scopes), &link),
                None => self.gap(
                    &id,
                    "scopes",
                    "Not every scope type that selects this context was established",
                    None,
                    vec![link],
                ),
            }

            selecting.insert(&context.id, scopes);
        }

        let incomplete_rows =
            "The rows of some contexts could not be read, so this list may be incomplete";
        let selected_scopes = |references: &[&LocalizationContextReference]| {
            let mut scopes = BTreeSet::new();

            for reference in references {
                scopes.extend(selecting.get(&reference.id)?.clone()?);
            }

            Some(scopes.into_iter().collect::<Vec<_>>())
        };

        for command in &declarations.commands {
            let id = self.subject(SubjectKind::LocalizationCommand, &command.name);
            let link = self.evidence(key, answer, &command.name, None)?;
            let references: Vec<_> = command.contexts.iter().collect();

            self.rule(&id, "existence", json!(true), &link);

            if !rows_complete {
                self.gap(&id, "contexts", incomplete_rows, None, vec![link.clone()]);
                self.gap(&id, "scopes", incomplete_rows, None, vec![link]);
                continue;
            }

            let context_ids: Vec<_> = references
                .iter()
                .map(|reference| context_subject(&contexts, reference))
                .collect();

            self.rule(&id, "contexts", json!(context_ids), &link);

            match selected_scopes(&references) {
                Some(scopes) => self.rule(&id, "scopes", json!(scopes), &link),
                None => self.gap(
                    &id,
                    "scopes",
                    "Not every scope type that selects the command's contexts was established",
                    None,
                    vec![link],
                ),
            }
        }

        let mut links: BTreeMap<&str, Vec<_>> = BTreeMap::new();

        for link_declaration in &declarations.links {
            links
                .entry(link_declaration.name.as_str())
                .or_default()
                .push(link_declaration);
        }

        for (name, rows) in links {
            let id = self.subject(SubjectKind::LocalizationLink, name);
            let link = self.evidence(
                key,
                answer,
                name,
                Some(GapSubject::LocalizationLink { name: name.into() }),
            )?;

            self.rule(&id, "existence", json!(true), &link);

            if !rows_complete {
                self.gap(
                    &id,
                    "alternatives",
                    incomplete_rows,
                    None,
                    vec![link.clone()],
                );
                self.gap(&id, "input_scopes", incomplete_rows, None, vec![link]);
                continue;
            }

            let alternatives: Option<Vec<_>> = rows
                .iter()
                .map(|row| {
                    let output = match &row.output {
                        LocalizationOutput::Listed(outputs) => json!(
                            outputs
                                .iter()
                                .map(|reference| context_subject(&contexts, reference))
                                .collect::<Vec<_>>()
                        ),
                        LocalizationOutput::Various => json!("various"),
                        LocalizationOutput::Unchanged => json!("unchanged"),
                        LocalizationOutput::Unresolved => return None,
                    };
                    let inputs: Vec<_> = row
                        .input_contexts
                        .iter()
                        .map(|reference| context_subject(&contexts, reference))
                        .collect();

                    Some(json!({"input_contexts": inputs, "output": output}))
                })
                .collect();

            match alternatives {
                Some(alternatives) => self.rule(&id, "alternatives", json!(alternatives), &link),
                None => self.gap(
                    &id,
                    "alternatives",
                    "Native did not follow the link to its output from every context that declares it",
                    None,
                    vec![link.clone()],
                ),
            }

            let inputs: Vec<_> = rows.iter().flat_map(|row| &row.input_contexts).collect();

            match selected_scopes(&inputs) {
                Some(scopes) => self.rule(&id, "input_scopes", json!(scopes), &link),
                None => self.gap(
                    &id,
                    "input_scopes",
                    "Not every scope type that selects the link's contexts was established",
                    None,
                    vec![link],
                ),
            }
        }

        let scope_subjects_by_id = self.scopes.clone();
        self.native_gaps(
            "localization",
            key,
            answer,
            &inventory,
            |subject| match subject {
                GapSubject::LocalizationLink { name } => Some(Target::Language(
                    SubjectKind::LocalizationLink,
                    name.clone(),
                )),
                GapSubject::LocalizationContext { id, .. } => contexts
                    .get(id)
                    .cloned()
                    .map(|name| Target::Language(SubjectKind::LocalizationContext, name)),
                GapSubject::ScopeType { id, .. } => scope_subjects_by_id
                    .get(id)
                    .cloned()
                    .map(|name| Target::Language(SubjectKind::Scope, name)),
                _ => None,
            },
        )
    }

    fn on_actions(&mut self, result: &Result<Answer<Vec<OnAction>>, Error>) -> Result<(), String> {
        let inventory = self.inventory("on_actions");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for on_action in &answer.value {
            self.callback(
                SubjectKind::OnAction,
                "on_actions",
                answer,
                &on_action.name,
                &on_action.entries,
            )?;
        }

        self.native_gaps("on_actions", "on_actions", answer, &inventory, |subject| {
            item_target(subject, SubjectKind::OnAction)
        })
    }

    fn game_rules(&mut self, result: &Result<Answer<Vec<GameRule>>, Error>) -> Result<(), String> {
        let inventory = self.inventory("game_rules");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for game_rule in &answer.value {
            let (id, link) = self.callback(
                SubjectKind::GameRule,
                "game_rules",
                answer,
                &game_rule.name,
                &game_rule.entries,
            )?;
            let kind = match game_rule.kind {
                RuleKind::Scripted => "scripted",
                RuleKind::Weighted => "weighted",
            };

            self.rule(&id, "kind", json!(kind), &link);
        }

        self.native_gaps("game_rules", "game_rules", answer, &inventory, |subject| {
            item_target(subject, SubjectKind::GameRule)
        })
    }

    /// Existence and entry scopes of an on_action or game rule.
    fn callback<T>(
        &mut self,
        kind: SubjectKind,
        key: &str,
        answer: &Answer<T>,
        name: &str,
        entries: &[EntryContext],
    ) -> Result<(String, EvidenceLink), String> {
        let id = self.subject(kind, name);
        let link = self.evidence(
            key,
            answer,
            name,
            Some(GapSubject::Item { name: name.into() }),
        )?;

        self.rule(&id, "existence", json!(true), &link);

        let established = if entries.is_empty() {
            Err("No call site of this name was followed".to_owned())
        } else if !link.native_gaps.is_empty() {
            Err("Native did not establish the entry scopes of every call site".to_owned())
        } else {
            self.entry_scopes(entries).ok_or_else(|| {
                format!(
                    "A call site supplies a self link or an unestablished scope, so what script sees there is not established: {}",
                    describe_entries(entries)
                )
            })
        };

        match established {
            Ok(scopes) => self.rule(&id, "entry_scopes", scopes, &link),
            Err(reason) => self.gap(
                &id,
                "entry_scopes",
                reason,
                Some(CALLBACK_CONTEXTS),
                vec![link.clone()],
            ),
        }

        Ok((id, link))
    }

    /// Every context's scopes, or `None` when one slot is not a scope type or `NotSet`.
    fn entry_scopes(&self, entries: &[EntryContext]) -> Option<Value> {
        let slot = |scope: &EntryScope| match scope {
            EntryScope::Scope(reference) => self
                .scopes
                .get(&reference.id)
                .map(|name| json!(SubjectKind::Scope.id(name))),
            EntryScope::NotSet => Some(json!("not_set")),
            _ => None,
        };
        let contexts: Option<Vec<_>> = entries
            .iter()
            .map(|entry| {
                let from: Option<Vec<_>> = entry.from.iter().map(slot).collect();

                Some(json!({
                    "this": slot(&entry.this)?,
                    "root": slot(&entry.root)?,
                    "from": from?,
                }))
            })
            .collect();

        contexts.map(Value::from)
    }

    fn defines(&mut self, result: &Result<Answer<Vec<Define>>, Error>) -> Result<(), String> {
        let inventory = self.inventory("defines");
        let Some(answer) = self.answered(&inventory, "answer", result) else {
            return Ok(());
        };

        for define in &answer.value {
            let name = format!("{}.{}", define.namespace, define.name);
            let id = self.subject(SubjectKind::Define, &name);
            let link = self.evidence(
                "defines",
                answer,
                &name,
                Some(GapSubject::Item { name: name.clone() }),
            )?;

            self.rule(&id, "existence", json!(true), &link);

            match value_type(define.value_type) {
                Some(value_type) => self.rule(&id, "value_type", json!(value_type), &link),
                None => self.gap(
                    &id,
                    "value_type",
                    "Native returned a value type that Atlas does not interpret",
                    None,
                    vec![link],
                ),
            }
        }

        self.native_gaps("defines", "defines", answer, &inventory, |subject| {
            item_target(subject, SubjectKind::Define)
        })
    }

    /// The loaded table is a content observation: only its counts become a rule, under the
    /// content that the game loaded. The names stay in the recorded answer for comparison.
    fn loaded_modifiers<T>(
        &mut self,
        session: &LoadedModifierSession,
        registries: &BTreeMap<String, T>,
    ) -> Result<(), String> {
        let inventory = self.inventory("loaded_modifiers");

        if !matches!(
            &session.disposal,
            Ok(Disposal::Confirmed | Disposal::NotApplicable)
        ) {
            self.gap(
                &inventory,
                "answer",
                format!(
                    "Native loaded-modifier session disposal failed: {:?}",
                    session.disposal
                ),
                None,
                Vec::new(),
            );
            return Ok(());
        }

        let Some(answer) = self.answered(&inventory, "answer", &session.observation) else {
            return Ok(());
        };
        let link = self.evidence("loaded_modifiers", answer, "summary", None)?;

        match loaded_summary(&answer.value) {
            Some((condition, summary)) => conditional_rule(
                self.snapshot,
                &inventory,
                "loaded_summary",
                vec![condition],
                summary,
                link,
            ),
            None => self.gap(
                &inventory,
                "loaded_summary",
                "Native returned a loaded content form that Atlas does not interpret",
                None,
                vec![link],
            ),
        }

        self.native_gaps(
            "loaded_modifiers",
            "loaded_modifiers",
            answer,
            &inventory,
            |subject| match subject {
                GapSubject::Registry { name } if registries.contains_key(name) => {
                    Some(Target::Existing(registry_id(name)))
                }
                _ => None,
            },
        )
    }
}

fn gap_kind(kind: GapKind) -> &'static str {
    match kind {
        GapKind::UnnamedRegistries => "unnamed_registries",
        GapKind::OutsideMethod => "outside_method",
        GapKind::UnreadableInput => "unreadable_input",
        GapKind::UnnamedField => "unnamed_field",
        GapKind::UnnamedDeclaration => "unnamed_declaration",
        GapKind::UnresolvedPath => "unresolved_path",
        GapKind::UnresolvedReader => "unresolved_reader",
        GapKind::ReaderSemantics => "reader_semantics",
        GapKind::IncompleteObservation => "incomplete_observation",
    }
}

/// Subject names of scope types: `{display name}/{keywords}`, so that the identity of one type
/// does not depend on the others. Two types that share both have no identity, so assembly fails.
fn scope_names(inventory: &ScopeInventory) -> Result<BTreeMap<ScopeId, String>, String> {
    let mut names = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for scope in &inventory.types {
        let name = format!("{}/{}", scope.name, scope.keywords.join(","));

        if !seen.insert(name.clone()) {
            return Err(format!(
                "Two scope types share the name and keywords {name}"
            ));
        }

        names.insert(scope.id.clone(), name);
    }

    Ok(names)
}

/// Subject names of localization contexts: the display name, or Native's build-scoped identity
/// when the name could not be read. Two contexts with one name have no identity, so assembly
/// fails.
fn context_names(
    declarations: &LocalizationDeclarations,
) -> Result<BTreeMap<LocalizationContextId, String>, String> {
    let mut names = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for context in &declarations.contexts {
        let name = if context.name.is_empty() {
            format!("@{}", opaque(&context.id))
        } else {
            context.name.clone()
        };

        if !seen.insert(name.clone()) {
            return Err(format!("Two localization contexts share the name {name}"));
        }

        names.insert(context.id.clone(), name);
    }

    Ok(names)
}

fn context_subject(
    contexts: &BTreeMap<LocalizationContextId, String>,
    reference: &LocalizationContextReference,
) -> String {
    let name = contexts
        .get(&reference.id)
        .cloned()
        .unwrap_or_else(|| format!("{}@{}", reference.name, opaque(&reference.id)));

    SubjectKind::LocalizationContext.id(&name)
}

fn opaque(id: &impl serde::Serialize) -> String {
    serde_json::to_value(id)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

struct Template {
    /// The name with `{key}` in place of the item key.
    text: String,
    /// The parts as Atlas publishes them, or `None` for a part that Atlas does not interpret.
    parts: Option<Value>,
}

fn name_template(parts: &[NamePart]) -> Template {
    let mut text = String::new();
    let mut published = Vec::new();
    let mut interpreted = true;

    for part in parts {
        match part {
            NamePart::Literal(literal) => {
                text.push_str(literal);
                published.push(json!({"literal": literal}));
            }
            NamePart::ItemKey => {
                text.push_str("{key}");
                published.push(json!("item_key"));
            }
            _ => {
                text.push_str("{?}");
                interpreted = false;
            }
        }
    }

    Template {
        text,
        parts: interpreted.then(|| json!(published)),
    }
}

fn value_type(value_type: DefineValueType) -> Option<&'static str> {
    Some(match value_type {
        DefineValueType::Boolean => "boolean",
        DefineValueType::Integer => "integer",
        DefineValueType::FixedPoint => "fixed_point",
        DefineValueType::Float => "float",
        DefineValueType::String => "string",
        DefineValueType::Vector => "vector",
        DefineValueType::List => "list",
        DefineValueType::Color => "color",
        DefineValueType::Date => "date",
        _ => return None,
    })
}

/// The content condition and counts that the snapshot records for a loaded modifier table, or
/// `None` for a content form that Atlas does not interpret.
pub(crate) fn loaded_summary(loaded: &LoadedModifiers) -> Option<(String, Value)> {
    let modifiers = &loaded.modifiers;
    let declared = modifiers
        .iter()
        .filter(|modifier| modifier.declared)
        .count();
    let generated = modifiers
        .iter()
        .filter(|modifier| !modifier.generated_by.is_empty())
        .count();
    let unexplained = modifiers
        .iter()
        .filter(|modifier| !modifier.declared && modifier.generated_by.is_empty())
        .count();
    let summary = json!({
        "total": modifiers.len(),
        "declared": declared,
        "generated": generated,
        "unexplained": unexplained,
    });

    Some((content_condition(&loaded.content)?, summary))
}

fn content_condition(content: &LoadedContent) -> Option<String> {
    match content {
        LoadedContent::Installation => Some("content:installation".into()),
        LoadedContent::Fixture { registry, files } => {
            Some(format!("content:fixture:{registry}:{}", files.join(",")))
        }
        _ => None,
    }
}

fn describe_entries(entries: &[EntryContext]) -> String {
    let slot = |scope: &EntryScope| match scope {
        EntryScope::Scope(reference) => reference.name.clone(),
        EntryScope::NotSet => "not_set".into(),
        EntryScope::SelfLink => "self_link".into(),
        EntryScope::Unresolved => "unresolved".into(),
        _ => "unknown".into(),
    };

    entries
        .iter()
        .map(|entry| {
            let from: Vec<_> = entry.from.iter().map(slot).collect();

            format!(
                "this={} root={} from=[{}]",
                slot(&entry.this),
                slot(&entry.root),
                from.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
