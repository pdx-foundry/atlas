//! Bounded tradition observations through Native's public API.

use pdx_native::{
    Answer, BuildId, Disposal, Error, Field, FixtureFieldQuestion, FixtureObservation,
    FixtureRequest, GameOptions, Native, Registry, Support,
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
}

impl Extraction {
    /// Whether every requested answer arrived and each game session closed safely.
    pub fn complete(&self) -> bool {
        self.registries.is_ok()
            && self.fields.values().all(Result::is_ok)
            && self
                .sessions
                .iter()
                .all(|session| session.observation.is_ok() && disposal_blocker(session).is_none())
    }
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

/// Ask Native the questions needed to assemble the first tradition snapshot.
pub async fn collect(native: &Native, options: impl Fn() -> GameOptions) -> Extraction {
    use pdx_native::Operation;

    let native_support = [
        Operation::Registries,
        Operation::RegistryFields,
        Operation::ObserveFixture,
    ]
    .into_iter()
    .map(|operation| {
        let name = match operation {
            Operation::Registries => "registries",
            Operation::RegistryFields => "registry_fields",
            Operation::RegistryItems => "registry_items",
            Operation::ObserveFixture => "observe_fixture",
            Operation::Declarations => "declarations",
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
        blocker = blocker.or_else(|| disposal_blocker(&session));
        sessions.push(session);
    }
    Extraction {
        build: native.build(),
        native_support,
        registries,
        fields,
        sessions,
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

fn disposal_blocker(session: &FixtureSession) -> Option<Error> {
    match &session.disposal {
        Ok(Disposal::Confirmed | Disposal::NotApplicable) => None,
        Ok(disposal @ Disposal::Unconfirmed(_)) => Some(Error::Startup {
            reason: format!("{} did not confirm game disposal", session.name),
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
