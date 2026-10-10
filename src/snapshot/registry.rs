//! Registry, field and fixture-outcome rules.

use super::scope::ScopeNames;
use super::{
    EvidenceLink, Rule, Snapshot, Subject, SubjectKind, argument_id, conditional_rule, evidence,
    field_id, gap, registry_id, rule,
};
use crate::extraction::Extraction;
use pdx_native::{
    AcceptedCategories, Answer, BlockFamily, Completeness, DerivedName, DiagnosticCoverage,
    DiagnosticJoin, Disposal, Field, FieldCondition, FieldMembers, FieldReadOutcome,
    FieldReference, FixtureFieldOutcome, FixtureObservation, FixtureStorage, Gap as NativeGap,
    GapKind, GapSubject, GrammarProperty, NameLookup, NamePart, ReaderKind, ReferenceTarget,
    RepeatBehavior,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Property families that registry assembly can establish.
pub(super) const PROPERTIES: &[&str] = &[
    "existence",
    "loader_path",
    "occurrences.parser_accepted",
    "value_form",
    "read",
    "shape",
    "repeat_behavior",
    "conditions",
    "block_family",
    "members",
    "uses",
    "scope_context",
    "read_scope",
    "reference",
    "accepted_categories",
];

pub(super) fn assemble(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    scopes: &ScopeNames,
) -> Result<(), String> {
    for registry in extraction.fields.keys() {
        assemble_registry(snapshot, extraction, registry)?;
        assemble_fields(snapshot, extraction, registry, scopes)?;
        assemble_derived_names(snapshot, extraction, registry)?;
    }
    assemble_outcomes(snapshot, extraction)
}

fn assemble_registry(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<(), String> {
    let id = registry_id(registry);
    snapshot.subjects.push(Subject {
        id: id.clone(),
        kind: SubjectKind::Registry,
        registry: Some(registry.into()),
        field: None,
        conditional: None,
        name: None,
    });
    match &extraction.registries {
        Ok(answer) if answer.value.iter().any(|found| found.name == registry) => {
            let evidence = evidence(
                snapshot,
                "registries",
                answer,
                format!("answers.registries:{registry}"),
                Some(GapSubject::Registry {
                    name: registry.into(),
                }),
            )?;
            rule(snapshot, &id, "existence", json!(true), evidence.clone());
            rule(snapshot, &id, "loader_path", json!(registry), evidence);
        }
        Ok(answer) => {
            let evidence = evidence(
                snapshot,
                "registries",
                answer,
                format!("answers.registries:{registry}"),
                Some(GapSubject::Registry {
                    name: registry.into(),
                }),
            )?;
            gap(
                snapshot,
                &id,
                "existence",
                "Native did not name this registry in the bounded answer",
                None,
                answer.gaps.clone(),
                vec![evidence.clone()],
            );
            gap(
                snapshot,
                &id,
                "loader_path",
                "Native did not establish this registry's content directory",
                None,
                answer.gaps.clone(),
                vec![evidence],
            );
        }
        Err(error) => {
            for property in ["existence", "loader_path"] {
                gap(
                    snapshot,
                    &id,
                    property,
                    format!(
                        "Native registry question failed: {}",
                        super::failure::error_reason(error)
                    ),
                    None,
                    Vec::new(),
                    Vec::new(),
                );
            }
        }
    }
    Ok(())
}

fn assemble_fields(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
    scopes: &ScopeNames,
) -> Result<(), String> {
    let registry_subject = registry_id(registry);
    let Some(result) = extraction.fields.get(registry) else {
        gap(
            snapshot,
            &registry_subject,
            "fields",
            "Native field answer is missing",
            None,
            Vec::new(),
            Vec::new(),
        );
        return Ok(());
    };
    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            gap(
                snapshot,
                &registry_subject,
                "fields",
                format!(
                    "Native field question failed: {}",
                    super::failure::error_reason(error)
                ),
                None,
                Vec::new(),
                Vec::new(),
            );
            return Ok(());
        }
    };
    if answer.completeness == Completeness::Partial {
        let evidence = evidence(
            snapshot,
            &format!("registry_fields/{registry}"),
            answer,
            format!("answers.fields.{registry}"),
            None,
        )?;
        gap(
            snapshot,
            &registry_subject,
            "fields_complete",
            "Native's root-field search is partial; undiscovered fields remain unknown",
            None,
            answer.gaps.clone(),
            vec![evidence],
        );
    }
    let names = derived_field_names(snapshot, extraction, registry)?;
    let fields = FieldSet::new(
        FieldOwner::Registry(registry),
        format!("registry_fields/{registry}"),
        format!("answers.fields.{registry}"),
        answer,
        &answer.value,
        scopes,
        names,
    );
    if !fields.unjoined_gaps.is_empty() {
        let link = evidence(snapshot, &fields.key, answer, fields.location.clone(), None)?;
        gap(
            snapshot,
            &registry_subject,
            "field_gap_subjects",
            format!(
                "Native field-gap names do not identify one field path: {}",
                fields.unjoined_names().join("; ")
            ),
            None,
            fields.unjoined_gaps.clone(),
            vec![link],
        );
    }
    fields.assemble(snapshot, &answer.value)
}

/// The lookups that a registry's derived names make with each field's text, by field path.
fn derived_field_names(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<BTreeMap<String, Vec<(Value, EvidenceLink)>>, String> {
    let mut lookups: BTreeMap<String, Vec<(Value, EvidenceLink)>> = BTreeMap::new();
    let Some(Ok(answer)) = extraction.derived_names.get(registry) else {
        return Ok(lookups);
    };

    for name in &answer.value {
        for part in &name.name {
            let NamePart::Field(path) = part else {
                continue;
            };
            let path = path.join("/");
            let link = evidence(
                snapshot,
                &format!("derived_names/{registry}"),
                answer,
                format!("answers.derived_names.{registry}:{path}"),
                None,
            )?;

            lookups
                .entry(path)
                .or_default()
                .push((json!({"lookup": name.lookup, "name": name.name}), link));
        }
    }

    Ok(lookups)
}

/// One subject for each name that a registry derives from the item key, or from one field's
/// text, with every way the engine uses it. A name that mixes a field with other parts has no
/// config naming line to join, so it gets no subject.
fn assemble_derived_names(
    snapshot: &mut Snapshot,
    extraction: &Extraction,
    registry: &str,
) -> Result<(), String> {
    let answer = match extraction.derived_names.get(registry) {
        Some(Ok(answer)) => answer,
        Some(Err(error)) => {
            gap(
                snapshot,
                &registry_id(registry),
                "derived_names",
                format!(
                    "Native derived-name question failed: {}",
                    super::failure::error_reason(error)
                ),
                None,
                Vec::new(),
                Vec::new(),
            );
            return Ok(());
        }
        None => return Ok(()),
    };
    let mut uses: BTreeMap<String, Vec<&DerivedName>> = BTreeMap::new();

    for name in &answer.value {
        let Some(rendered) = render_name(&name.name) else {
            continue;
        };
        let lookup = match name.lookup {
            NameLookup::Localization => "localization",
            NameLookup::Sprite => "sprite",
            NameLookup::File => "file",
            _ => continue,
        };

        uses.entry(format!("{registry}/{lookup}/{rendered}"))
            .or_default()
            .push(name);
    }

    for (name, uses) in uses {
        let id = SubjectKind::DerivedName.id(&name);
        snapshot.subjects.push(Subject {
            id: id.clone(),
            kind: SubjectKind::DerivedName,
            registry: None,
            field: None,
            conditional: None,
            name: Some(name.clone()),
        });
        let link = evidence(
            snapshot,
            &format!("derived_names/{registry}"),
            answer,
            format!("answers.derived_names.{registry}:{name}"),
            None,
        )?;
        let uses: Vec<_> = uses
            .iter()
            .map(|name| {
                json!({
                    "stage": name.stage,
                    "on_missing": name.on_missing,
                    "condition": name.condition,
                })
            })
            .collect();

        rule(snapshot, &id, "derived_name", json!(uses), link);
    }

    Ok(())
}

/// A derived name as a config naming line writes it: `$` for the item key, or `{field:a/b}` for
/// a name that is one field's text. `None` for a name that mixes a field with other parts.
fn render_name(parts: &[NamePart]) -> Option<String> {
    if let [NamePart::Field(path)] = parts {
        return Some(format!("{{field:{}}}", path.join("/")));
    }

    parts
        .iter()
        .map(|part| match part {
            NamePart::Literal(text) => Some(text.as_str()),
            NamePart::ItemKey => Some("$"),
            _ => None,
        })
        .collect()
}

/// Whose subjects a set of Native fields describes.
#[derive(Clone, Copy)]
pub(super) enum FieldOwner<'a> {
    /// Root and nested fields of a content directory.
    Registry(&'a str),
    /// Named keys of a command's block, by the command's subject identity.
    Command(&'a str),
}

impl FieldOwner<'_> {
    fn subject(self, path: &str, conditional: bool) -> Subject {
        match self {
            Self::Registry(registry) => Subject {
                id: field_id(registry, path),
                kind: SubjectKind::Field,
                registry: Some(registry.into()),
                field: Some(path.into()),
                conditional: Some(conditional),
                name: None,
            },
            Self::Command(command) => Subject {
                id: argument_id(command, path),
                kind: SubjectKind::Argument,
                registry: None,
                field: Some(path.into()),
                conditional: Some(conditional),
                name: Some(command.into()),
            },
        }
    }
}

/// The fields of one Native answer, with what their facets need: their Native gaps by path,
/// scope names and derived-name lookups.
pub(super) struct FieldSet<'a, T> {
    owner: FieldOwner<'a>,
    key: String,
    location: String,
    answer: &'a Answer<T>,
    scopes: &'a ScopeNames,
    /// Native gaps that name one field path, by that path.
    field_gaps: BTreeMap<String, Vec<NativeGap>>,
    /// Field gaps whose name matches no path or several.
    unjoined_gaps: Vec<NativeGap>,
    /// Paths of each field name, for naming the unjoined gaps.
    leaf_paths: BTreeMap<String, Vec<String>>,
    /// Derived-name lookups of each field's text, by field path.
    names: BTreeMap<String, Vec<(Value, EvidenceLink)>>,
}

impl<'a, T> FieldSet<'a, T> {
    /// Index `fields` of `answer`, recorded under answer key `key`.
    pub(super) fn new(
        owner: FieldOwner<'a>,
        key: String,
        location: String,
        answer: &'a Answer<T>,
        fields: &[Field],
        scopes: &'a ScopeNames,
        names: BTreeMap<String, Vec<(Value, EvidenceLink)>>,
    ) -> Self {
        let mut leaf_paths = BTreeMap::new();
        field_paths(fields, "", &mut leaf_paths);
        let full_paths: BTreeSet<_> = leaf_paths.values().flatten().cloned().collect();
        let mut field_gaps: BTreeMap<String, Vec<NativeGap>> = BTreeMap::new();
        let mut unjoined_gaps = Vec::new();

        for native_gap in &answer.gaps {
            let Some(names) = gap_path(native_gap.subject.as_ref()) else {
                continue;
            };
            let path = match names.as_slice() {
                // Native names a nested field by its full path, so one name is a root field;
                // without one, a name that only one nested path ends with is that field.
                [name] if full_paths.contains(name) => Some(name.clone()),
                [name] => leaf_paths
                    .get(name)
                    .filter(|paths| paths.len() == 1)
                    .map(|paths| paths[0].clone()),
                // A path below a field's own members, such as a weight block's keys, names
                // that field: Atlas publishes no subject below it.
                _ => (1..=names.len())
                    .rev()
                    .map(|length| names[..length].join("/"))
                    .find(|path| full_paths.contains(path)),
            };

            match path {
                Some(path) => field_gaps.entry(path).or_default().push(native_gap.clone()),
                None => unjoined_gaps.push(native_gap.clone()),
            }
        }

        Self {
            owner,
            key,
            location,
            answer,
            scopes,
            field_gaps,
            unjoined_gaps,
            leaf_paths,
            names,
        }
    }

    fn unjoined_names(&self) -> Vec<String> {
        let names: BTreeSet<_> = self
            .unjoined_gaps
            .iter()
            .filter_map(|native_gap| gap_path(native_gap.subject.as_ref()))
            .map(|names| {
                let name = names.join(".");
                let paths = match names.as_slice() {
                    [leaf] => self.leaf_paths.get(leaf).map(|paths| paths.join(", ")),
                    _ => None,
                };

                format!(
                    "{name}: {}",
                    paths.unwrap_or_else(|| "no discovered field".into())
                )
            })
            .collect();

        names.into_iter().collect()
    }

    /// Publish each field and its nested fields.
    pub(super) fn assemble(&self, snapshot: &mut Snapshot, fields: &[Field]) -> Result<(), String> {
        for field in fields {
            self.assemble_field(snapshot, &field.name, field, false)?;
        }

        Ok(())
    }

    fn assemble_field(
        &self,
        snapshot: &mut Snapshot,
        path: &str,
        field: &Field,
        parent_conditional: bool,
    ) -> Result<(), String> {
        let unconditional_outcome = match field.read.as_slice() {
            [alternative]
                if !parent_conditional && alternative.condition == FieldCondition::Always =>
            {
                Some(&alternative.outcome)
            }
            _ => None,
        };
        let conditional = unconditional_outcome.is_none();
        let subject = self.owner.subject(path, conditional);
        let id = subject.id.clone();
        snapshot.subjects.push(subject);
        let mut evidence = evidence(
            snapshot,
            &self.key,
            self.answer,
            format!("{}:{path}", self.location),
            None,
        )?;
        evidence.native_gaps = self.field_gaps.get(path).cloned().unwrap_or_default();
        rule(snapshot, &id, "existence", json!(true), evidence.clone());
        if field.read.is_empty() {
            return Err(format!("Field {id} has no read alternatives"));
        }
        rule(snapshot, &id, "read", json!(field.read), evidence.clone());
        rule(snapshot, &id, "shape", json!(field.shape), evidence.clone());
        rule(
            snapshot,
            &id,
            "conditions",
            json!(
                field
                    .read
                    .iter()
                    .map(|alternative| &alternative.condition)
                    .collect::<Vec<_>>()
            ),
            evidence.clone(),
        );
        rule(
            snapshot,
            &id,
            "uses",
            json!({"stage":"stored_value_selection", "selections":field.uses}),
            evidence.clone(),
        );
        rule(
            snapshot,
            &id,
            "block_family",
            json!(
                field
                    .read
                    .iter()
                    .map(|alternative| {
                        let family = match &alternative.outcome {
                            FieldReadOutcome::Read { reader, .. } => reader.family,
                            _ => BlockFamily::Unknown,
                        };
                        json!({"condition":alternative.condition, "family":family})
                    })
                    .collect::<Vec<_>>()
            ),
            evidence.clone(),
        );
        gap(
            snapshot,
            &id,
            "domain",
            "Native reports this field property as Unknown",
            Some("field_semantics"),
            Vec::new(),
            vec![evidence.clone()],
        );
        rule(
            snapshot,
            &id,
            "members",
            json!(field.members),
            evidence.clone(),
        );
        if let FieldMembers::Fields(children) = &field.members {
            for child in children {
                self.assemble_field(
                    snapshot,
                    &format!("{path}/{}", child.name),
                    child,
                    conditional,
                )?;
            }
        }
        let block =
            assemble_field_value_form(snapshot, &id, field, &evidence, unconditional_outcome)?;
        if block {
            self.assemble_block(snapshot, &id, field, &evidence);
        }
        assemble_field_repeat(snapshot, &id, unconditional_outcome, &evidence);
        self.assemble_reference(snapshot, &id, path, field, &evidence);
        assemble_accepted_categories(snapshot, &id, field, &evidence);
        gap(
            snapshot,
            &id,
            "occurrences.minimum",
            "No validation result establishes that this field is required",
            None,
            Vec::new(),
            vec![evidence],
        );
        Ok(())
    }

    /// The nested grammar and scope context of an unconditional block reader.
    fn assemble_block(
        &self,
        snapshot: &mut Snapshot,
        id: &str,
        field: &Field,
        evidence: &EvidenceLink,
    ) {
        let family = field.reader.family;
        let members_established = matches!(
            field.members,
            FieldMembers::Fields(_)
                | FieldMembers::ModifierBlock(_)
                | FieldMembers::WeightBlock(_)
                | FieldMembers::TriggeredModifier(_)
        );
        if !matches!(family, BlockFamily::Trigger | BlockFamily::Effect) && !members_established {
            gap(
                snapshot,
                id,
                "nested_grammar",
                "Native establishes neither the block's command family nor its members",
                None,
                Vec::new(),
                vec![evidence.clone()],
            );
        }
        // A compiler checks a modifier block's keys by its accepted categories, not by scope.
        if !is_modifier_block(field) {
            self.assemble_scope_context(snapshot, id, field, evidence);
        }
    }

    fn assemble_scope_context(
        &self,
        snapshot: &mut Snapshot,
        id: &str,
        field: &Field,
        evidence: &EvidenceLink,
    ) {
        let this = match &field.read_scope {
            GrammarProperty::Known(alternatives) if !alternatives.is_empty() => {
                self.scopes.read_scope(alternatives)
            }
            _ => None,
        };
        let context_gap = evidence.native_gaps.iter().any(|native_gap| {
            matches!(
                native_gap.kind,
                GapKind::UnresolvedPath | GapKind::UnreadableInput | GapKind::UnnamedDeclaration
            )
        });
        let contexts = if field.entry_contexts.is_empty() || context_gap {
            None
        } else {
            self.scopes.entry_scopes(&field.entry_contexts)
        };
        match &this {
            Some(this) => rule(snapshot, id, "read_scope", this.clone(), evidence.clone()),
            None => gap(
                snapshot,
                id,
                "read_scope",
                "Native did not establish the scope that this block's keys are read in",
                None,
                evidence.native_gaps.clone(),
                vec![evidence.clone()],
            ),
        }
        let reason = match (this, contexts) {
            (Some(this), Some(contexts)) => {
                rule(
                    snapshot,
                    id,
                    "scope_context",
                    json!({"this": this, "contexts": contexts}),
                    evidence.clone(),
                );
                return;
            }
            (Some(_), None) => {
                "Native establishes the read-time `this`, but not every evaluation context of `root`, `from` and `prev`"
            }
            (None, Some(_)) => {
                "Native establishes the evaluation contexts, but not the read-time `this`"
            }
            (None, None) => {
                "Native establishes neither the read-time `this` nor the evaluation contexts"
            }
        };
        gap(
            snapshot,
            id,
            "scope_context",
            reason,
            None,
            evidence.native_gaps.clone(),
            vec![evidence.clone()],
        );
    }

    /// What a field's text is looked up in: its reference lookups and the derived names that
    /// use it.
    fn assemble_reference(
        &self,
        snapshot: &mut Snapshot,
        id: &str,
        path: &str,
        field: &Field,
        evidence: &EvidenceLink,
    ) {
        let lookups = match &field.reference {
            FieldReference::Lookups(lookups) => lookups.as_slice(),
            _ => &[],
        };
        let names = self.names.get(path).map(Vec::as_slice).unwrap_or_default();
        let string_like = matches!(
            field.reader.kind,
            ReaderKind::String | ReaderKind::Reference
        );
        if lookups.is_empty() && names.is_empty() && !string_like {
            return;
        }
        let unresolved = lookups.iter().any(|lookup| {
            !matches!(
                lookup.target,
                ReferenceTarget::Registry { .. } | ReferenceTarget::Triggers
            )
        });
        let reason = if unresolved {
            "Native did not join a lookup of this field to the collection it searches"
        } else if lookups.is_empty() && names.is_empty() {
            "A string-like reader does not establish a lookup category"
        } else {
            let mut links = vec![evidence.clone()];
            links.extend(names.iter().map(|(_, link)| link.clone()));
            conditional_rule(
                snapshot,
                id,
                "reference",
                Vec::new(),
                json!({
                    "lookups": lookups,
                    "derived_names": names.iter().map(|(name, _)| name).collect::<Vec<_>>(),
                }),
                links,
            );
            return;
        };
        gap(
            snapshot,
            id,
            "reference",
            reason,
            None,
            Vec::new(),
            vec![evidence.clone()],
        );
    }
}

/// The field path that a Native gap names, as names from the outermost key.
fn gap_path(subject: Option<&GapSubject>) -> Option<Vec<String>> {
    match subject? {
        GapSubject::Field { name } => Some(name.split('.').map(Into::into).collect()),
        GapSubject::KeyPath { path } => Some(path.clone()),
        _ => None,
    }
}

fn field_paths(fields: &[Field], parent: &str, paths: &mut BTreeMap<String, Vec<String>>) {
    for field in fields {
        let path = if parent.is_empty() {
            field.name.clone()
        } else {
            format!("{parent}/{}", field.name)
        };
        paths
            .entry(field.name.clone())
            .or_default()
            .push(path.clone());
        if let FieldMembers::Fields(children) = &field.members {
            field_paths(children, &path, paths);
        }
    }
}

/// The categories of modifiers that a modifier reader accepts. An unresolved answer is a gap only
/// on a modifier block: Native also leaves it unresolved for fields whose reader is unknown.
fn assemble_accepted_categories(
    snapshot: &mut Snapshot,
    id: &str,
    field: &Field,
    evidence: &EvidenceLink,
) {
    match &field.accepted_categories {
        AcceptedCategories::NotApplicable => {}
        AcceptedCategories::Listed(categories) => rule(
            snapshot,
            id,
            "accepted_categories",
            json!(categories),
            evidence.clone(),
        ),
        AcceptedCategories::Enclosing => rule(
            snapshot,
            id,
            "accepted_categories",
            json!("enclosing"),
            evidence.clone(),
        ),
        _ if is_modifier_block(field) => gap(
            snapshot,
            id,
            "accepted_categories",
            "Native did not establish which modifier categories this reader accepts",
            None,
            evidence.native_gaps.clone(),
            vec![evidence.clone()],
        ),
        _ => {}
    }
}

fn is_modifier_block(field: &Field) -> bool {
    field.reader.family == BlockFamily::Modifier
        || matches!(field.members, FieldMembers::ModifierBlock(_))
}

/// Publish the value form, and return whether the field is an unconditional block reader.
fn assemble_field_value_form(
    snapshot: &mut Snapshot,
    id: &str,
    field: &Field,
    evidence: &EvidenceLink,
    outcome: Option<&FieldReadOutcome>,
) -> Result<bool, String> {
    let (reader, reason, owner) = match outcome {
        Some(FieldReadOutcome::Read { reader, .. }) => (Some(reader), "", None),
        Some(FieldReadOutcome::Rejected) => (
            None,
            "Native rejects this field on the unconditional loader path",
            None,
        ),
        Some(_) => (
            None,
            "Native did not establish the unconditional reader's value form",
            None,
        ),
        None => (
            None,
            "Conditional alternatives do not establish an unconditional value form; unresolved branches remain unknown",
            Some("field_conditions"),
        ),
    };
    let Some(reader) = reader else {
        rule(
            snapshot,
            id,
            "value_form",
            json!({"alternatives":field.read}),
            evidence.clone(),
        );
        gap(
            snapshot,
            id,
            "value_form.unresolved",
            reason,
            owner,
            evidence.native_gaps.clone(),
            vec![evidence.clone()],
        );
        return Ok(false);
    };
    let reader_id = &reader.id;
    let reader_id = reader_id
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| error.to_string())?;
    let shape = match reader.kind {
        ReaderKind::Boolean => Some(("boolean", json!("boolean"))),
        ReaderKind::Integer => Some(("integer", json!("integer"))),
        ReaderKind::FixedPoint | ReaderKind::Float => Some(("number", json!("number"))),
        // A literal number, or a script value or variable that the scope supplies.
        ReaderKind::ScopedNumeric => Some(("scoped_number", json!(["number", "string"]))),
        ReaderKind::String => Some(("string", json!("string"))),
        ReaderKind::Reference => Some(("reference", json!("string"))),
        ReaderKind::Block => {
            rule(
                snapshot,
                id,
                "value_form",
                json!({"form":"block","reader":reader_id}),
                evidence.clone(),
            );
            return Ok(true);
        }
        _ => None,
    };
    let Some((form, schema_type)) = shape else {
        gap(
            snapshot,
            id,
            "value_form",
            "Native did not establish this reader's value form",
            None,
            evidence.native_gaps.clone(),
            vec![evidence.clone()],
        );
        return Ok(false);
    };
    let schema_name = format!(
        "reader-{}",
        reader_id.as_ref().and_then(Value::as_str).unwrap_or(form)
    );
    let schema = json!({"type":schema_type});
    if let Some(existing) = snapshot.schemas.definitions.get(&schema_name) {
        if existing != &schema {
            return Err(format!("Conflicting shapes for reader {reader_id:?}"));
        }
    } else {
        snapshot
            .schemas
            .definitions
            .insert(schema_name.clone(), schema);
    }
    rule(
        snapshot,
        id,
        "value_form",
        json!({"form":form,"schema":format!("#/schemas/$defs/{schema_name}"),"reader":reader_id}),
        evidence.clone(),
    );
    Ok(false)
}

fn assemble_field_repeat(
    snapshot: &mut Snapshot,
    id: &str,
    outcome: Option<&FieldReadOutcome>,
    evidence: &EvidenceLink,
) {
    match outcome {
        Some(FieldReadOutcome::Read { shape, .. })
            if matches!(
                shape.repeat,
                RepeatBehavior::Replace | RepeatBehavior::Accumulate | RepeatBehavior::Merges
            ) =>
        {
            rule(
                snapshot,
                id,
                "repeat_behavior",
                json!(shape.repeat),
                evidence.clone(),
            );
        }
        _ => gap(
            snapshot,
            id,
            "occurrences.maximum",
            "Native did not establish unconditional repeat behavior",
            None,
            Vec::new(),
            vec![evidence.clone()],
        ),
    }
}

#[derive(Default)]
struct Occurrences {
    counts: BTreeSet<usize>,
    final_values: BTreeSet<String>,
    rejected: Vec<Value>,
    evidence: Vec<EvidenceLink>,
    gap_evidence: Vec<EvidenceLink>,
    reasons: BTreeMap<&'static str, Vec<String>>,
}

enum ParserOutcome {
    Accepted {
        count: usize,
        final_value: Option<&'static str>,
    },
    Rejected(Vec<Value>),
    Unavailable {
        property: &'static str,
        reason: String,
    },
}

fn classify_parser_outcome(
    observation: &FixtureObservation,
    outcome: &FixtureFieldOutcome,
) -> ParserOutcome {
    let diagnostics: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter_map(|index| observation.diagnostics.get(*index))
        .collect();
    if diagnostics.len() != outcome.diagnostics.len() {
        return ParserOutcome::Unavailable {
            property: "validation",
            reason: "Native's diagnostic index did not resolve".into(),
        };
    }
    if observation
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.join, DiagnosticJoin::Unavailable(_)))
    {
        return ParserOutcome::Unavailable {
            property: "validation",
            reason: "A fixture diagnostic was not joined to a source location".into(),
        };
    }
    if !diagnostics.is_empty() {
        return ParserOutcome::Rejected(diagnostics.into_iter().map(|diagnostic| {
            json!({"definition":outcome.question.definition,"text":diagnostic.text,"join":diagnostic.join})
        }).collect());
    }
    match &observation.diagnostic_coverage {
        DiagnosticCoverage::Complete { .. } => {}
        DiagnosticCoverage::Unavailable(reason) => {
            return ParserOutcome::Unavailable {
                property: "validation",
                reason: reason.clone(),
            };
        }
        DiagnosticCoverage::NotRequested => {
            return ParserOutcome::Unavailable {
                property: "validation",
                reason: "Parser diagnostics were not requested".into(),
            };
        }
    }
    match &outcome.storage {
        FixtureStorage::Observed {
            occurrences,
            final_value,
            completeness: Completeness::Complete,
        } => {
            let final_value = final_value.as_ref().and_then(|final_value| {
                let first = occurrences.first().map(|record| &record.value);
                let last = occurrences.last().map(|record| &record.value);
                if last == Some(final_value) {
                    Some("last")
                } else if first == Some(final_value) {
                    Some("first")
                } else {
                    None
                }
            });
            ParserOutcome::Accepted {
                count: occurrences.len(),
                final_value,
            }
        }
        FixtureStorage::Observed {
            completeness: Completeness::Partial,
            ..
        } => ParserOutcome::Unavailable {
            property: "storage",
            reason: "Parser storage was partial".into(),
        },
        FixtureStorage::Unavailable(reason) => ParserOutcome::Unavailable {
            property: "storage",
            reason: reason.clone(),
        },
    }
}

fn assemble_outcomes(snapshot: &mut Snapshot, extraction: &Extraction) -> Result<(), String> {
    let mut outcomes: BTreeMap<String, Occurrences> = BTreeMap::new();
    for session in &extraction.sessions {
        if !matches!(
            &session.disposal,
            Ok(Disposal::Confirmed | Disposal::NotApplicable)
        ) {
            gap(
                snapshot,
                &registry_id(session.registry),
                &format!("fixture.{}", session.name),
                format!(
                    "Native fixture session disposal failed: {}",
                    super::failure::session_reason(&session.disposal)
                ),
                None,
                Vec::new(),
                Vec::new(),
            );
            continue;
        }
        let observation = match &session.observation {
            Ok(answer) => answer,
            Err(error) => {
                gap(
                    snapshot,
                    &registry_id(session.registry),
                    &format!("fixture.{}", session.name),
                    format!(
                        "Native fixture question failed: {}",
                        super::failure::error_reason(error)
                    ),
                    None,
                    Vec::new(),
                    Vec::new(),
                );
                continue;
            }
        };
        for outcome in &observation.value.field_outcomes {
            let id = field_id(&outcome.question.registry, &outcome.question.field);
            if !snapshot.subjects.iter().any(|subject| subject.id == id) {
                continue;
            }
            let entry = outcomes.entry(id.clone()).or_default();
            let outcome_evidence = evidence(
                snapshot,
                &format!("observe_fixture/{}", session.name),
                observation,
                format!(
                    "answers.sessions.{}.fixture:{}:{}",
                    session.name, outcome.question.definition, outcome.question.field
                ),
                None,
            )?;
            entry.gap_evidence.push(outcome_evidence.clone());
            match classify_parser_outcome(&observation.value, outcome) {
                ParserOutcome::Accepted { count, final_value } => {
                    entry.counts.insert(count);
                    if let Some(final_value) = final_value {
                        entry.final_values.insert(final_value.into());
                    }
                    entry.evidence.push(outcome_evidence.clone());
                }
                ParserOutcome::Rejected(rejected) => {
                    entry.rejected.extend(rejected);
                    entry.evidence.push(outcome_evidence.clone());
                }
                ParserOutcome::Unavailable { property, reason } => {
                    entry.reasons.entry(property).or_default().push(reason)
                }
            }
        }
    }
    for (id, mut entry) in outcomes {
        entry.rejected.sort_by_key(Value::to_string);
        entry.rejected.dedup();
        if !entry.counts.is_empty() {
            let mut answer = json!({"stage":"parser_storage","counts":entry.counts,"rejected_inputs":entry.rejected});
            if entry.final_values.len() == 1 {
                answer["final_value"] = json!(entry.final_values.first().expect("one value"));
            }
            snapshot.rules.push(Rule {
                id: format!("{id}#occurrences.parser_accepted"),
                subject: id.clone(),
                property: "occurrences.parser_accepted".into(),
                conditions: Vec::new(),
                answer,
                evidence: entry.evidence,
            });
        } else {
            gap(
                snapshot,
                &id,
                "occurrences.parser_accepted",
                if entry.reasons.is_empty() {
                    "No diagnostic-free parser storage outcome was established".into()
                } else {
                    entry
                        .reasons
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                },
                None,
                Vec::new(),
                entry.gap_evidence.clone(),
            );
        }
        for (property, reasons) in entry.reasons {
            gap(
                snapshot,
                &id,
                property,
                reasons.join("; "),
                None,
                Vec::new(),
                entry.gap_evidence.clone(),
            );
        }
    }
    let answered: BTreeSet<_> = snapshot
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .chain(snapshot.gaps.iter().map(|gap| gap.id.as_str()))
        .collect();
    let missing: Vec<_> = snapshot
        .subjects
        .iter()
        .filter(|subject| subject.kind == SubjectKind::Field)
        .filter(|subject| {
            !answered.contains(format!("{}#occurrences.parser_accepted", subject.id).as_str())
        })
        .map(|subject| subject.id.clone())
        .collect();
    for id in missing {
        let evidence = snapshot
            .rules
            .iter()
            .find(|rule| rule.id == format!("{id}#existence"))
            .map(|rule| rule.evidence.clone())
            .unwrap_or_default();
        gap(
            snapshot,
            &id,
            "occurrences.parser_accepted",
            "No established fixture recipe supplies a valid definition and observation phase for this field",
            None,
            Vec::new(),
            evidence,
        );
    }
    Ok(())
}
