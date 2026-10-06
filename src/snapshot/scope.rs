//! Script-visible scope subjects for Native's scope references.

use super::SubjectKind;
use pdx_native::{EntryContext, EntryScope, ReadScope, ScopeId, ScopeInventory, ScopeReference};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The scope subject name of each scope type in Native's scope inventory.
#[derive(Default)]
pub(super) struct ScopeNames {
    /// `{display name}/{keywords joined by ","}` of each scope type.
    pub(super) names: BTreeMap<ScopeId, String>,
}

impl ScopeNames {
    /// Name every scope type; two types with one name and keyword set have no identity.
    pub(super) fn new(inventory: &ScopeInventory) -> Result<Self, String> {
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

        Ok(Self { names })
    }

    /// Scope subject identities for `references`, or `None` when one is not in the inventory.
    pub(super) fn ids(&self, references: &[ScopeReference]) -> Option<Vec<String>> {
        references
            .iter()
            .map(|reference| {
                self.names
                    .get(&reference.id)
                    .map(|name| SubjectKind::Scope.id(name))
            })
            .collect()
    }

    /// The alternatives of a read-time `this`: scope subjects, or `enclosing` for the parent's
    /// scope. `None` when a scope type is not in the inventory.
    pub(super) fn read_scope(&self, alternatives: &[ReadScope]) -> Option<Value> {
        let mut scopes = Vec::new();

        for alternative in alternatives {
            match alternative {
                ReadScope::Enclosing => scopes.push(json!("enclosing")),
                ReadScope::Types(references) => {
                    scopes.extend(self.ids(references)?.into_iter().map(Value::from));
                }
            }
        }

        Some(Value::from(scopes))
    }

    /// Every context's script-visible scopes, or `None` when a scope type is unknown.
    pub(super) fn entry_scopes(&self, entries: &[EntryContext]) -> Option<Value> {
        let contexts: Option<Vec<_>> = entries
            .iter()
            .map(|entry| {
                let this = self.entry_scope(&entry.this)?;
                let root = match entry.root {
                    EntryScope::SelfLink => this.clone(),
                    _ => self.entry_scope(&entry.root)?,
                };

                Some(json!({
                    "this": this,
                    "root": root,
                    "from": self.entry_chain(&entry.from)?,
                    "prev": self.entry_chain(&entry.prev)?,
                }))
            })
            .collect();

        contexts.map(Value::from)
    }

    fn entry_scope(&self, scope: &EntryScope) -> Option<Value> {
        match scope {
            EntryScope::Scope(reference) => self
                .names
                .get(&reference.id)
                .map(|name| json!(SubjectKind::Scope.id(name))),
            EntryScope::NotSet => Some(json!("not_set")),
            // `this` cannot establish its type by referring to itself.
            _ => None,
        }
    }

    /// Apply Native's hand-checked self-link assumption to the reported chain positions.
    fn entry_chain(&self, chain: &[EntryScope]) -> Option<Vec<Value>> {
        let mut scopes = Vec::new();

        for scope in chain {
            let value = match scope {
                EntryScope::SelfLink => scopes.last().cloned().unwrap_or(json!("not_set")),
                _ => self.entry_scope(scope)?,
            };

            scopes.push(value);
        }

        Some(scopes)
    }
}
