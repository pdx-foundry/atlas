//! A frozen Atlas question flow over Native's public answers.

mod questions;

use pdx_native::{
    Answer, BuildId, Completeness, DiagnosticCoverage, DiagnosticJoin, DiagnosticWindow, Disposal,
    Error, Field, FixtureFieldQuestion, FixtureObservation, FixtureRequest, FixtureRuntime,
    FixtureStorage, GameOptions, GameReadiness, Gap, Native, Operation, ProcessingStage,
    ReaderKind, Registry, Support,
};
use questions::{CATEGORIES, Check, Promise, Question, TRADITIONS, questions};
use serde::Serialize;
use std::collections::BTreeMap;

/// The complete answers, support report, and question-level Atlas coverage.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Opaque identity of the exact build that supplied the answers.
    pub build: BuildId,
    /// Native availability, independent of Atlas rule coverage.
    pub native_support: BTreeMap<String, Support>,
    /// Native answers kept whole, with errors retained per question.
    pub answers: Answers,
    /// One resolution for every Atlas question mapped to the tradition config.
    pub coverage: Vec<Resolution>,
    /// Count of question-level outcomes, never a whole-category validity claim.
    pub summary: Summary,
}

/// Static and live Native answers; no Atlas conclusion replaces an answer.
#[derive(Debug, Serialize)]
pub struct Answers {
    /// Discovered registries.
    pub registries: Result<Answer<Vec<Registry>>, Error>,
    /// Root fields for each requested registry.
    pub fields: BTreeMap<String, Result<Answer<Vec<Field>>, Error>>,
    /// Independent game sessions, including each cleanup result.
    pub sessions: Vec<SessionReport>,
}

/// Answers obtained while one game, or one recorded stand-in, was open.
#[derive(Debug, Serialize)]
pub struct SessionReport {
    /// Atlas's stable name for the request.
    pub name: &'static str,
    /// The game's startup boundary, absent on a failed start.
    pub readiness: Option<GameReadiness>,
    /// Registry item answers requested in this session.
    pub items: BTreeMap<String, Result<Answer<Vec<String>>, Error>>,
    /// Prepared fixture answer, if this session had a fixture.
    pub fixture: Option<Result<Answer<FixtureObservation>, Error>>,
    /// Close result, or the startup error if no game was returned.
    pub termination: Result<Disposal, Error>,
}

/// One config question and what the available Native evidence establishes.
#[derive(Debug, Serialize)]
pub struct Resolution {
    /// Stable Atlas question name.
    pub question: String,
    /// Path in `config/common/traditions.cwt`, or an explicitly engine-only field.
    pub config_rule: &'static str,
    /// Which first-release promise needs the answer.
    promise: Promise,
    /// One observation, owned gap, or unanswered Native question.
    pub outcome: Outcome,
}

/// Atlas's interpretation of exactly one question.
#[derive(Debug, Serialize)]
pub enum Outcome {
    /// The named Native answer contains the specific observation.
    Observed {
        /// Path to the Native answer that established this observation.
        evidence: String,
    },
    /// Native did not establish the required rule property.
    Gap {
        /// Ticket that owns this missing observation.
        owner: &'static str,
        /// Why this observation is not established.
        reason: String,
        /// Native's typed gaps, when the answer supplied them.
        native_gaps: Vec<Gap>,
    },
    /// A Native question failed or its recorded answer is absent.
    Unanswered {
        /// The original Native error, or an explicit missing-outcome error.
        error: Error,
    },
}

/// Question-level totals for one report.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    /// Questions with specific established evidence.
    pub observed: usize,
    /// Questions with a named owner for missing evidence.
    pub gaps: usize,
    /// Questions whose Native answer failed or was not recorded.
    pub unanswered: usize,
}

/// Run the same bounded questions against a real game or recorded answers.
///
/// The options closure is ignored by recorded Native sessions, which start no process.
pub async fn run(native: &Native, options: impl Fn() -> GameOptions) -> Report {
    let native_support = [
        Operation::Registries,
        Operation::RegistryFields,
        Operation::RegistryItems,
        Operation::ObserveFixture,
    ]
    .into_iter()
    .map(|operation| (format!("{operation:?}"), native.supports(operation)))
    .collect();
    let registries = native.registries();
    let tree_registry = registries.as_ref().ok().and_then(|answer| {
        answer
            .value
            .iter()
            .find(|registry| {
                registry.name.contains("tree_template") || registry.name.contains("tradition_tree")
            })
            .map(|registry| registry.name.clone())
    });
    let fields = [TRADITIONS, CATEGORIES]
        .into_iter()
        .map(|registry| (registry.into(), native.registry_fields(registry)))
        .collect();
    let mut baseline_names = vec![TRADITIONS.into(), CATEGORIES.into()];
    if let Some(name) = tree_registry {
        baseline_names.push(name);
    }
    let sessions = vec![
        session(native, &options, "baseline", None, baseline_names).await,
        session(
            native,
            &options,
            "tradition_outcomes",
            Some(tradition_fixture()),
            vec![],
        )
        .await,
        session(
            native,
            &options,
            "category_outcomes",
            Some(category_fixture()),
            vec![],
        )
        .await,
        session(
            native,
            &options,
            "category_reads",
            Some(FixtureRequest::new(
                "common/tradition_categories/atlas_category.txt",
                crate::CATEGORY_FIXTURE,
            )),
            vec![],
        )
        .await,
    ];
    let answers = Answers {
        registries,
        fields,
        sessions,
    };
    let coverage = resolve(&answers);
    let mut summary = Summary::default();
    for row in &coverage {
        match row.outcome {
            Outcome::Observed { .. } => summary.observed += 1,
            Outcome::Gap { .. } => summary.gaps += 1,
            Outcome::Unanswered { .. } => summary.unanswered += 1,
        }
    }
    Report {
        build: native.build(),
        native_support,
        answers,
        coverage,
        summary,
    }
}

async fn session(
    native: &Native,
    options: &impl Fn() -> GameOptions,
    name: &'static str,
    fixture: Option<FixtureRequest>,
    item_names: Vec<String>,
) -> SessionReport {
    let has_fixture = fixture.is_some();
    let mut launch = options();
    if let Some(fixture) = fixture {
        launch = launch.fixture(fixture);
    }
    let mut game = match native.start_game(launch).await {
        Ok(game) => game,
        Err(error) => {
            return SessionReport {
                name,
                readiness: None,
                items: item_names
                    .into_iter()
                    .map(|name| (name, Err(error.clone())))
                    .collect(),
                fixture: has_fixture.then(|| Err(error.clone())),
                termination: Err(error),
            };
        }
    };
    let readiness = Some(game.readiness());
    let fixture = if has_fixture {
        Some(game.observe_fixture().await)
    } else {
        None
    };
    let mut items = BTreeMap::new();
    for item_name in item_names {
        items.insert(item_name.clone(), game.registry_items(&item_name).await);
    }
    let termination = game.close().await;
    SessionReport {
        name,
        readiness,
        items,
        fixture,
        termination,
    }
}

/// Prepared valid, omitted, repeated, malformed, and unknown-field tradition inputs.
pub fn tradition_fixture() -> FixtureRequest {
    let text = include_str!("../fixtures/frozen-traditions.txt");
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

/// Prepared category input that asks for each discovered root field.
pub fn category_fixture() -> FixtureRequest {
    let text = include_str!("../fixtures/frozen-categories.txt");
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

/// Resolve Atlas questions independently; an unresolved field blocks only dependent questions.
pub fn resolve(answers: &Answers) -> Vec<Resolution> {
    questions()
        .into_iter()
        .map(|question| {
            let outcome = evaluate(&question, answers);
            Resolution {
                question: question.id,
                config_rule: question.rule,
                promise: question.promise,
                outcome,
            }
        })
        .collect()
}

fn evaluate(question: &Question, answers: &Answers) -> Outcome {
    match question.check {
        Check::Registry(name) => match &answers.registries {
            Err(error) => unanswered(error),
            Ok(answer) if answer.value.iter().any(|registry| registry.name == name) => {
                observed(format!("answers.registries:{name}"))
            }
            Ok(answer) => missing_from_answer(
                question,
                answer.completeness,
                &answer.gaps,
                "registry was not named",
            ),
        },
        Check::Items(name) => match items(answers, "baseline", name) {
            Err(error) => unanswered(&error),
            Ok(answer) if answer.completeness == Completeness::Complete => {
                observed(format!("answers.sessions.baseline.items:{name}"))
            }
            Ok(answer) => gap(question, "item list did not complete", &answer.gaps),
        },
        Check::Field(registry, field) => match fields(answers, registry) {
            Err(error) => unanswered(&error),
            Ok(answer) if answer.value.iter().any(|item| item.name == field) => {
                observed(format!("answers.fields.{registry}:{field}"))
            }
            Ok(answer) => missing_from_answer(
                question,
                answer.completeness,
                &answer.gaps,
                "field was not discovered",
            ),
        },
        Check::Reader(registry, field) => match fields(answers, registry) {
            Err(error) => unanswered(&error),
            Ok(answer) => match answer.value.iter().find(|item| item.name == field) {
                Some(item)
                    if item.reader.id.is_some() && item.reader.kind != ReaderKind::Unknown =>
                {
                    observed(format!("answers.fields.{registry}:{field}.reader"))
                }
                Some(_) => gap(
                    question,
                    "reader identity or kind is unresolved",
                    &gaps_for_field(answer, field),
                ),
                None => gap(
                    question,
                    "field was not discovered, so its reader is unknown",
                    &answer.gaps,
                ),
            },
        },
        Check::SharedReader => match fields(answers, TRADITIONS) {
            Err(error) => unanswered(&error),
            Ok(answer) => {
                let agenda = answer
                    .value
                    .iter()
                    .find(|item| item.name == "unlocks_agenda");
                let tooltip = answer
                    .value
                    .iter()
                    .find(|item| item.name == "custom_tooltip");
                match (agenda, tooltip) {
                    (Some(agenda), Some(tooltip))
                        if agenda.reader.id.is_some() && agenda.reader.id == tooltip.reader.id =>
                    {
                        observed(
                            "answers.fields.common/traditions:unlocks_agenda+custom_tooltip".into(),
                        )
                    }
                    _ => gap(question, "shared reader was not established", &answer.gaps),
                }
            }
        },
        Check::Storage(session_name, definition, field) => match fixture(answers, session_name) {
            Err(error) => unanswered(&error),
            Ok(answer) => match answer.value.field_outcomes.iter().find(|outcome| {
                outcome.question.definition == definition && outcome.question.field == field
            }) {
                None => unanswered(&observation_error("requested field outcome is absent")),
                Some(outcome) => match &outcome.storage {
                    FixtureStorage::String {
                        completeness: Completeness::Complete,
                        ..
                    } => observed(format!(
                        "answers.sessions.{session_name}.fixture:{definition}.{field}.storage"
                    )),
                    FixtureStorage::String { .. } => {
                        unanswered(&observation_error("field storage window is incomplete"))
                    }
                    FixtureStorage::Unavailable(reason) => {
                        gap(question, reason, &gaps_for_field(answer, field))
                    }
                },
            },
        },
        Check::DiagnosticCoverage(session_name) => match fixture(answers, session_name) {
            Err(error) => unanswered(&error),
            Ok(answer)
                if matches!(
                    answer.value.diagnostic_coverage,
                    DiagnosticCoverage::Complete {
                        window: DiagnosticWindow::FixtureFileLoad
                    }
                ) =>
            {
                observed(format!(
                    "answers.sessions.{session_name}.fixture.diagnostic_coverage"
                ))
            }
            Ok(answer) => gap(
                question,
                "parser diagnostic window is incomplete",
                &answer.gaps,
            ),
        },
        Check::Diagnostic(text) => match fixture(answers, "tradition_outcomes") {
            Err(error) => unanswered(&error),
            Ok(answer) => {
                if !matches!(
                    answer.value.diagnostic_coverage,
                    DiagnosticCoverage::Complete {
                        window: DiagnosticWindow::FixtureFileLoad
                    }
                ) {
                    return gap(
                        question,
                        "parser diagnostic window is incomplete",
                        &answer.gaps,
                    );
                }
                if answer.value.diagnostics.iter().any(|diagnostic| {
                    diagnostic.text.contains(text)
                        && matches!(diagnostic.join, DiagnosticJoin::Source { .. })
                }) {
                    observed(format!(
                        "answers.sessions.tradition_outcomes.fixture.diagnostics:{text}"
                    ))
                } else {
                    gap(
                        question,
                        format!("no source-joined {text} diagnostic"),
                        &answer.gaps,
                    )
                }
            }
        },
        Check::Runtime => match fixture(answers, "tradition_outcomes") {
            Err(error) => unanswered(&error),
            Ok(answer) => match answer.value.field_outcomes.iter().find(|outcome| {
                outcome.question.definition == "atlas_valid"
                    && outcome.question.field == "unlocks_agenda"
            }) {
                Some(outcome) if matches!(outcome.runtime, FixtureRuntime::Unavailable(_)) => gap(
                    question,
                    "runtime is outside the initial-load method",
                    &answer.gaps,
                ),
                Some(_) => unanswered(&observation_error("runtime dimension was not requested")),
                None => unanswered(&observation_error("requested runtime outcome is absent")),
            },
        },
        Check::CategoryRead(field) => match fixture(answers, "category_reads") {
            Err(error) => unanswered(&error),
            Ok(answer)
                if answer.value.field_reads.iter().any(|read| {
                    read.field == field && read.stage == ProcessingStage::FieldReadEntry
                }) =>
            {
                observed(format!(
                    "answers.sessions.category_reads.fixture.field_reads:{field}"
                ))
            }
            Ok(answer) => missing_from_answer(
                question,
                answer.completeness,
                &answer.gaps,
                "category field read was not observed",
            ),
        },
        Check::RegistrationEntries => match fixture(answers, "category_reads") {
            Err(error) => unanswered(&error),
            Ok(answer) if !answer.value.registration_entries.is_empty() => {
                observed("answers.sessions.category_reads.fixture.registration_entries".into())
            }
            Ok(answer) => missing_from_answer(
                question,
                answer.completeness,
                &answer.gaps,
                "registration entry was not observed",
            ),
        },
        Check::TreeTemplateRegistry => match &answers.registries {
            Err(error) => unanswered(error),
            Ok(answer) => {
                let found = answer.value.iter().find(|registry| {
                    registry.name.contains("tree_template")
                        || registry.name.contains("tradition_tree")
                });
                match found {
                    None => gap(
                        question,
                        "no tree-template registry was named",
                        &answer.gaps,
                    ),
                    Some(registry) => match items(answers, "baseline", &registry.name) {
                        Err(error) => unanswered(&error),
                        Ok(items) if items.completeness == Completeness::Complete => {
                            observed(format!("answers.sessions.baseline.items:{}", registry.name))
                        }
                        Ok(items) => gap(
                            question,
                            "tree-template items did not complete",
                            &items.gaps,
                        ),
                    },
                }
            }
        },
        Check::Missing(reason) => gap(question, reason, &[]),
    }
}

fn fields<'a>(answers: &'a Answers, registry: &str) -> Result<&'a Answer<Vec<Field>>, Error> {
    answers
        .fields
        .get(registry)
        .ok_or_else(|| observation_error("requested registry fields are absent"))?
        .as_ref()
        .map_err(Clone::clone)
}

fn items<'a>(
    answers: &'a Answers,
    session_name: &str,
    registry: &str,
) -> Result<&'a Answer<Vec<String>>, Error> {
    answers
        .sessions
        .iter()
        .find(|session| session.name == session_name)
        .and_then(|session| session.items.get(registry))
        .ok_or_else(|| observation_error("requested item answer is absent"))?
        .as_ref()
        .map_err(Clone::clone)
}

fn fixture<'a>(
    answers: &'a Answers,
    session_name: &str,
) -> Result<&'a Answer<FixtureObservation>, Error> {
    answers
        .sessions
        .iter()
        .find(|session| session.name == session_name)
        .and_then(|session| session.fixture.as_ref())
        .ok_or_else(|| observation_error("requested fixture answer is absent"))?
        .as_ref()
        .map_err(Clone::clone)
}

fn observation_error(reason: &str) -> Error {
    Error::Observation {
        operation: Operation::ObserveFixture,
        reason: reason.into(),
    }
}

fn gaps_for_field<T>(answer: &Answer<T>, field: &str) -> Vec<Gap> {
    answer
        .gaps
        .iter()
        .filter(|gap| gap.subject.as_deref() == Some(field))
        .cloned()
        .collect()
}

fn observed(evidence: String) -> Outcome {
    Outcome::Observed { evidence }
}
fn unanswered(error: &Error) -> Outcome {
    Outcome::Unanswered {
        error: error.clone(),
    }
}
fn gap(question: &Question, reason: impl Into<String>, native_gaps: &[Gap]) -> Outcome {
    Outcome::Gap {
        owner: question.owner,
        reason: reason.into(),
        native_gaps: native_gaps.to_vec(),
    }
}
fn missing_from_answer(
    question: &Question,
    completeness: Completeness,
    native_gaps: &[Gap],
    reason: &str,
) -> Outcome {
    if completeness == Completeness::Partial {
        gap(
            question,
            format!("{reason}; Native answer is partial"),
            native_gaps,
        )
    } else {
        gap(question, reason, native_gaps)
    }
}

/// Compare live and recorded answers while ignoring only basis and session lifecycle.
pub fn comparable(report: &Report) -> serde_json::Value {
    let mut value = serde_json::to_value((&report.answers, &report.coverage))
        .expect("report answers serialize");
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("basis") {
                    object.insert("basis".into(), serde_json::Value::String("Recorded".into()));
                }
                object.remove("readiness");
                object.remove("termination");
                for child in object.values_mut() {
                    normalize(child);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    normalize(item);
                }
            }
            _ => {}
        }
    }
    normalize(&mut value);
    value
}
