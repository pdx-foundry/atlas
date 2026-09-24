//! Bounded tradition observations and language declarations through Native's public API.

use pdx_native::{
    Answer, BuildId, Declaration, DeclarationKind, Define, Disposal, Error, Field,
    FixtureFieldQuestion, FixtureObservation, FixtureRequest, GameOptions, GameRule,
    LoadedModifiers, LocalizationDeclarations, ModifierCategory, ModifierDeclaration,
    ModifierFamily, Native, OnAction, Registry, ScopeInventory, ScopeLink, Support,
};
use std::collections::BTreeMap;

/// The tradition definition directory.
pub const TRADITIONS: &str = "common/traditions";
/// The tradition category definition directory.
pub const CATEGORIES: &str = "common/tradition_categories";

/// Native answers retained without reinterpretation.
#[derive(Debug)]
pub struct Extraction {
    /// Exact build reported by Native.
    pub build: BuildId,
    /// Native's operation availability.
    pub native_support: BTreeMap<String, Support>,
    /// Discovered content directories.
    pub registries: Result<Answer<Vec<Registry>>, Error>,
    /// Root fields of each requested directory.
    pub fields: BTreeMap<String, Result<Answer<Vec<Field>>, Error>>,
    /// Independent fixture sessions and their disposal results.
    pub sessions: Vec<FixtureSession>,
    /// Static language declarations.
    pub language: Language,
    /// The loaded modifier table and its session's disposal result.
    pub loaded_modifiers: LoadedModifierSession,
}

impl Extraction {
    /// Whether every requested answer arrived and each game session closed safely.
    pub fn complete(&self) -> bool {
        self.registries.is_ok()
            && self.fields.values().all(Result::is_ok)
            && self.sessions.iter().all(|session| {
                session.observation.is_ok()
                    && disposal_blocker(session.name, &session.disposal).is_none()
            })
            && self.language.complete()
            && self.loaded_modifiers.observation.is_ok()
            && disposal_blocker(LOADED_MODIFIERS, &self.loaded_modifiers.disposal).is_none()
    }
}

/// Native's static answers about the script language, retained without reinterpretation.
#[derive(Debug)]
pub struct Language {
    /// Effect declarations.
    pub effects: Result<Answer<Vec<Declaration>>, Error>,
    /// Trigger declarations.
    pub triggers: Result<Answer<Vec<Declaration>>, Error>,
    /// Directly declared modifiers.
    pub modifiers: Result<Answer<Vec<ModifierDeclaration>>, Error>,
    /// Declared modifier category names.
    pub modifier_categories: Result<Answer<Vec<ModifierCategory>>, Error>,
    /// Generated modifier families of each discovered registry.
    pub modifier_families: BTreeMap<String, Result<Answer<Vec<ModifierFamily>>, Error>>,
    /// Scope types and keyword groups.
    pub scopes: Result<Answer<ScopeInventory>, Error>,
    /// Scope links.
    pub scope_links: Result<Answer<Vec<ScopeLink>>, Error>,
    /// Localization contexts, commands and links.
    pub localization: Result<Answer<LocalizationDeclarations>, Error>,
    /// On_actions and their entry contexts.
    pub on_actions: Result<Answer<Vec<OnAction>>, Error>,
    /// Game rules and their entry contexts.
    pub game_rules: Result<Answer<Vec<GameRule>>, Error>,
    /// Define names and value types.
    pub defines: Result<Answer<Vec<Define>>, Error>,
}

impl Language {
    fn ask(native: &Native, registries: &[String]) -> Self {
        Self {
            effects: native.declarations(DeclarationKind::Effect),
            triggers: native.declarations(DeclarationKind::Trigger),
            modifiers: native.modifiers(),
            modifier_categories: native.modifier_categories(),
            modifier_families: registries
                .iter()
                .map(|registry| (registry.clone(), native.modifier_families(registry)))
                .collect(),
            scopes: native.scopes(),
            scope_links: native.scope_links(),
            localization: native.localization_declarations(),
            on_actions: native.on_actions(),
            game_rules: native.game_rules(),
            defines: native.defines(),
        }
    }

    fn complete(&self) -> bool {
        self.effects.is_ok()
            && self.triggers.is_ok()
            && self.modifiers.is_ok()
            && self.modifier_categories.is_ok()
            && self.modifier_families.values().all(Result::is_ok)
            && self.scopes.is_ok()
            && self.scope_links.is_ok()
            && self.localization.is_ok()
            && self.on_actions.is_ok()
            && self.game_rules.is_ok()
            && self.defines.is_ok()
    }
}

const LOADED_MODIFIERS: &str = "loaded_modifiers";

/// The loaded modifier answer and its session's disposal result.
#[derive(Debug)]
pub struct LoadedModifierSession {
    /// Native's loaded modifier answer, or why it failed.
    pub observation: Result<Answer<LoadedModifiers>, Error>,
    /// Confirmation that the game process is gone.
    pub disposal: Result<Disposal, Error>,
}

/// One fixture answer and its session's disposal result.
#[derive(Debug)]
pub struct FixtureSession {
    /// Stable name for the bounded request.
    pub name: &'static str,
    /// Registry containing the authored fixture.
    pub registry: &'static str,
    /// Native's complete or failed fixture answer.
    pub observation: Result<Answer<FixtureObservation>, Error>,
    /// Confirmation that the game process is gone.
    pub disposal: Result<Disposal, Error>,
}

/// Ask Native the questions needed to assemble the registry and language snapshot.
pub async fn collect(native: &Native, options: impl Fn() -> GameOptions) -> Extraction {
    use pdx_native::Operation;

    let native_support = [
        Operation::Registries,
        Operation::RegistryFields,
        Operation::ObserveFixture,
        Operation::Declarations,
        Operation::Modifiers,
        Operation::ModifierCategories,
        Operation::ModifierFamilies,
        Operation::Scopes,
        Operation::ScopeLinks,
        Operation::LocalizationDeclarations,
        Operation::OnActions,
        Operation::GameRules,
        Operation::Defines,
        Operation::LoadedModifiers,
    ]
    .into_iter()
    .map(|operation| {
        let name = match operation {
            Operation::Registries => "registries",
            Operation::RegistryFields => "registry_fields",
            Operation::RegistryItems => "registry_items",
            Operation::ObserveFixture => "observe_fixture",
            Operation::Declarations => "declarations",
            Operation::Modifiers => "modifiers",
            Operation::ModifierCategories => "modifier_categories",
            Operation::ModifierFamilies => "modifier_families",
            Operation::Scopes => "scopes",
            Operation::ScopeLinks => "scope_links",
            Operation::LocalizationDeclarations => "localization_declarations",
            Operation::OnActions => "on_actions",
            Operation::GameRules => "game_rules",
            Operation::Defines => "defines",
            Operation::LoadedModifiers => "loaded_modifiers",
        };
        (name.into(), native.supports(operation))
    })
    .collect();
    let registries = native.registries();
    let fields = match &registries {
        Ok(answer) => answer
            .value
            .iter()
            .map(|registry| {
                (
                    registry.name.clone(),
                    native.registry_fields(&registry.name),
                )
            })
            .collect(),
        Err(_) => BTreeMap::new(),
    };
    let discovered: std::collections::BTreeSet<_> = fields.keys().map(String::as_str).collect();

    let requests = [
        ("tradition_outcomes", TRADITIONS, tradition_fixture()),
        ("category_outcomes", CATEGORIES, category_fixture()),
        (
            "category_reads",
            CATEGORIES,
            FixtureRequest::new(
                "common/tradition_categories/atlas_category.txt",
                include_str!("../fixtures/category-reads.txt"),
            ),
        ),
    ];
    let mut sessions = Vec::with_capacity(requests.len());
    let mut blocker: Option<Error> = None;
    for (name, registry, request) in requests
        .into_iter()
        .filter(|(_, registry, _)| discovered.contains(registry))
    {
        let session = match &blocker {
            Some(error) => FixtureSession {
                name,
                registry,
                observation: Err(error.clone()),
                disposal: Err(error.clone()),
            },
            None => run_fixture(native, options(), name, registry, request).await,
        };
        blocker = blocker.or_else(|| disposal_blocker(session.name, &session.disposal));
        sessions.push(session);
    }
    let registry_names: Vec<_> = fields.keys().cloned().collect();
    let language = Language::ask(native, &registry_names);
    let loaded_modifiers = match blocker {
        Some(error) => LoadedModifierSession {
            observation: Err(error.clone()),
            disposal: Err(error),
        },
        None => read_loaded_modifiers(native, options()).await,
    };
    Extraction {
        build: native.build(),
        native_support,
        registries,
        fields,
        sessions,
        language,
        loaded_modifiers,
    }
}

/// Start a game that reads the loaded modifier table, ask for it, and close the game.
///
/// With recorded answers no process starts, so `options` only needs a placeholder supervisor.
pub async fn read_loaded_modifiers(native: &Native, options: GameOptions) -> LoadedModifierSession {
    let mut game = match native.start_game(options.loaded_modifiers()).await {
        Ok(game) => game,
        Err(error) => {
            return LoadedModifierSession {
                observation: Err(error.clone()),
                disposal: Err(error),
            };
        }
    };
    let observation = game.loaded_modifiers().await;
    let disposal = game.close().await;
    LoadedModifierSession {
        observation,
        disposal,
    }
}

async fn run_fixture(
    native: &Native,
    options: GameOptions,
    name: &'static str,
    registry: &'static str,
    request: FixtureRequest,
) -> FixtureSession {
    let mut game = match native.start_game(options.fixture(request)).await {
        Ok(game) => game,
        Err(error) => {
            return FixtureSession {
                name,
                registry,
                observation: Err(error.clone()),
                disposal: Err(error),
            };
        }
    };
    let observation = game.observe_fixture().await;
    let disposal = game.close().await;
    FixtureSession {
        name,
        registry,
        observation,
        disposal,
    }
}

fn disposal_blocker(name: &str, disposal: &Result<Disposal, Error>) -> Option<Error> {
    match disposal {
        Ok(Disposal::Confirmed | Disposal::NotApplicable) => None,
        Ok(disposal @ Disposal::Unconfirmed(_)) => Some(Error::Startup {
            reason: format!("{name} did not confirm game disposal"),
            disposal: disposal.clone(),
        }),
        Err(error) => Some(error.clone()),
    }
}

fn tradition_fixture() -> FixtureRequest {
    let text = include_str!("../fixtures/traditions.txt");
    let mut asked: Vec<_> = [
        "unlocks_agenda",
        "modifier",
        "triggered_modifier",
        "possible",
        "potential",
        "on_enabled",
        "on_disabled",
        "custom_tooltip",
        "custom_tooltip_with_modifiers",
        "tradition_swap",
        "ai_weight",
    ]
    .into_iter()
    .map(|field| {
        let question = FixtureFieldQuestion::new(TRADITIONS, "atlas_valid", field);
        if field == "unlocks_agenda" {
            question.with_runtime()
        } else {
            question
        }
    })
    .collect();
    for definition in [
        "atlas_omitted",
        "atlas_repeated",
        "atlas_malformed",
        "atlas_unknown",
    ] {
        asked.push(FixtureFieldQuestion::new(
            TRADITIONS,
            definition,
            "unlocks_agenda",
        ));
    }
    FixtureRequest::field_outcomes("common/traditions/atlas_frozen.txt", text, asked)
}

fn category_fixture() -> FixtureRequest {
    let text = include_str!("../fixtures/categories.txt");
    let asked = [
        "desc",
        "tree_template",
        "adoption_bonus",
        "finish_bonus",
        "traditions",
        "potential",
        "ai_weight",
    ]
    .into_iter()
    .map(|field| FixtureFieldQuestion::new(CATEGORIES, "atlas_category", field));
    FixtureRequest::field_outcomes("common/tradition_categories/atlas_frozen.txt", text, asked)
}
