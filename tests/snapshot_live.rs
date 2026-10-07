use pdx_atlas::{extraction, snapshot};
use pdx_native::{Basis, Disposal, GameOptions, Native, Support};
use std::{collections::BTreeMap, process::Command};

fn recorded_basis(mut snapshot: snapshot::Snapshot) -> snapshot::Snapshot {
    let mut sources = BTreeMap::new();
    let mut keys = BTreeMap::new();
    for (old, mut source) in snapshot.sources {
        source.basis = Basis::Recorded;
        let new = format!("{}@recorded", source.method);
        keys.insert(old, new.clone());
        sources.insert(new, source);
    }
    snapshot.sources = sources;
    for answer in snapshot.answers.values_mut() {
        answer.source = keys[&answer.source].clone();
    }
    snapshot
}

#[tokio::test]
#[ignore = "requires STELLARIS_PATH with the exact supported build"]
async fn live_and_recorded_snapshots_match_after_basis_normalization() {
    let installation = std::env::var_os("STELLARIS_PATH").expect("STELLARIS_PATH is required");
    let recording = tempfile::tempdir().unwrap();
    let live = Native::open(installation)
        .unwrap()
        .record_answers_to(recording.path());
    let live_answers = extraction::collect(&live, || {
        let mut supervisor = Command::new(env!("CARGO_BIN_EXE_pdx-atlas"));
        supervisor.arg("--supervisor");
        GameOptions::new(supervisor)
    })
    .await;
    assert!(
        live_answers
            .sessions
            .iter()
            .all(|session| session.disposal == Ok(Disposal::Confirmed)),
        "{:#?}",
        live_answers.sessions
    );
    assert_eq!(
        live_answers.loaded_modifiers.disposal,
        Ok(Disposal::Confirmed),
        "{:#?}",
        live_answers.loaded_modifiers.observation.as_ref().err()
    );
    assert!(live_answers.complete());
    let live_snapshot = snapshot::assemble(&live_answers).unwrap();
    // The recorded fixture keeps a sample; the live run checks the build's whole population.
    assert_eq!(live_snapshot.coverage.registries.len(), 164);

    let replay = Native::from_recorded_answers(recording.path()).unwrap();
    let replay_answers =
        extraction::collect(&replay, || GameOptions::new(Command::new("unused"))).await;
    assert!(
        replay_answers
            .sessions
            .iter()
            .all(|session| session.disposal == Ok(Disposal::NotApplicable))
    );
    assert_eq!(
        replay_answers.loaded_modifiers.disposal,
        Ok(Disposal::NotApplicable)
    );
    let replay_snapshot = snapshot::assemble(&replay_answers).unwrap();
    assert!(
        live_snapshot
            .coverage
            .native_support
            .values()
            .all(|support| *support == Support::Supported)
    );
    let unsupported: Vec<_> = replay_snapshot
        .coverage
        .native_support
        .iter()
        .filter(|(_, support)| matches!(support, Support::Unsupported(_)))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        unsupported,
        [
            "check_script",
            "dynamic_names",
            "modifier_category_keys",
            "modifier_nodes",
            "script_expansions"
        ]
    );
    let mut normalized_live = recorded_basis(live_snapshot);
    normalized_live.coverage.native_support = replay_snapshot.coverage.native_support.clone();
    normalized_live.snapshot = replay_snapshot.snapshot.clone();
    assert_eq!(
        snapshot::json_bytes(&normalized_live).unwrap(),
        snapshot::json_bytes(&replay_snapshot).unwrap()
    );
}
