//! Atlas's bounded category-fixture consumer. Native owns all engine mechanisms.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use pdx_native::{Answer, Disposal, Error, GameOptions, GameReadiness, Native, Registry};
use serde::Serialize;
use std::collections::BTreeMap;

/// The frozen first-release tradition and category question flow.
pub mod frozen;

/// Content directories asked for, in request order.
pub const REGISTRIES: [&str; 2] = ["common/traditions", "common/tradition_categories"];
/// An authored input, never proof that the engine accepted it.
pub const CATEGORY_FIXTURE: &str = include_str!("../fixtures/category.txt");

/// Whether the answer includes the fixture's category key.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum FixtureKey {
    /// The returned items include the key; fields and validation remain unknown.
    Observed,
    /// The returned items omit the key, with no wider absence claim.
    NotObservedInAnswer,
    /// The question concerns another registry.
    Unknown,
}

/// A Native answer kept whole, alongside Atlas's bounded interpretation.
#[derive(Debug, Serialize)]
pub struct Observation {
    /// Values, completeness, typed gaps and source stamp, unchanged.
    pub native: Answer<Vec<String>>,
    /// A statement about these items only, also for partial and recorded answers.
    pub fixture_key: FixtureKey,
    /// Fixture questions registry enumeration cannot establish.
    pub gaps: Vec<&'static str>,
}

/// Interpret live and recorded answers by the same rules; keep their basis unchanged.
pub fn process_observation(registry: &str, native: Answer<Vec<String>>) -> Observation {
    let fixture_key = if registry != "common/tradition_categories" {
        FixtureKey::Unknown
    } else if native.value.iter().any(|key| key == "atlas_early_category") {
        FixtureKey::Observed
    } else {
        FixtureKey::NotObservedInAnswer
    };
    Observation {
        native,
        fixture_key,
        gaps: vec![
            "Fixture execution is not requested by registry enumeration",
            "Fixture tree_template storage and traditions relationships are unknown",
            "Engine validation and runtime behavior are unknown",
            "Schema completeness and rule coverage are unknown",
        ],
    }
}

/// Static registry discovery; no game starts.
pub fn describe(native: &Native) -> Result<Answer<Vec<Registry>>, Error> {
    native.registries()
}

/// Independent query results and the separate session disposal.
#[derive(Debug, Serialize)]
pub struct Observations {
    /// Present only when start_game returned a game; never gameplay readiness.
    pub readiness: Option<GameReadiness>,
    /// Each requested answer or error; one error does not suppress another question.
    pub queries: BTreeMap<String, Result<Observation, Error>>,
    /// Close result, or the startup error including disposal when Native established it.
    pub termination: Result<Disposal, Error>,
}

/// Read both registries independently and always close a returned game.
/// Recorded backends use this same flow, with no supervisor or process.
pub async fn collect(native: &Native, options: GameOptions) -> Observations {
    let mut game = match native.start_game(options).await {
        Ok(game) => game,
        Err(error) => {
            return Observations {
                readiness: None,
                queries: BTreeMap::new(),
                termination: Err(error),
            };
        }
    };
    let readiness = Some(game.readiness());
    let mut queries = BTreeMap::new();
    for registry in REGISTRIES {
        let answer = game
            .registry_items(registry)
            .await
            .map(|answer| process_observation(registry, answer));
        queries.insert(registry.into(), answer);
    }
    let termination = game.close().await;
    Observations {
        readiness,
        queries,
        termination,
    }
}
