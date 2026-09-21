use pdx_atlas::{extraction, snapshot};
use pdx_native::{Basis, Disposal, GameOptions, Native};
use std::{collections::BTreeMap, process::Command};

fn recorded_basis(mut snapshot: snapshot::Snapshot) -> snapshot::Snapshot {
    let mut sources = BTreeMap::new();
    let mut keys = BTreeMap::new();
    for (old, mut source) in snapshot.sources {
        source.basis = Basis::Recorded;
        let new = format!("{}@Recorded", source.method);
        keys.insert(old, new.clone());
        sources.insert(new, source);
    }
    snapshot.sources = sources;
    for rule in &mut snapshot.rules {
        for evidence in &mut rule.evidence {
            evidence.source = keys[&evidence.source].clone();
        }
    }
    for gap in &mut snapshot.gaps {
        for evidence in &mut gap.evidence {
            evidence.source = keys[&evidence.source].clone();
        }
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
    let live_snapshot = snapshot::assemble(&live_answers).unwrap();

    let replay = Native::from_recorded_answers(recording.path()).unwrap();
    let replay_answers =
        extraction::collect(&replay, || GameOptions::new(Command::new("unused"))).await;
    assert!(
        replay_answers
            .sessions
            .iter()
            .all(|session| session.disposal == Ok(Disposal::NotApplicable))
    );
    let replay_snapshot = snapshot::assemble(&replay_answers).unwrap();
    assert_eq!(
        snapshot::json_bytes(&recorded_basis(live_snapshot)).unwrap(),
        snapshot::json_bytes(&replay_snapshot).unwrap()
    );
}
