use pdx_atlas::{extraction, snapshot};
use pdx_native::{Disposal, Error, GameOptions, Native};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn recording() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();

    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());

        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A copy of the sample recording that a test may change.
fn editable_recording() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let answers = root.path().join("answers");
    copy_tree(&recording(), &answers);

    (root, answers)
}

fn unused() -> GameOptions {
    GameOptions::new(Command::new("unused"))
}

/// Static answers from the sample recording, joined with the observations of `observations`.
async fn static_mode(observations: &Path) -> Result<extraction::Extraction, String> {
    let installation = Native::from_recorded_answers(recording()).unwrap();
    let observed = Native::from_recorded_answers(observations).unwrap();
    let static_answers = extraction::collect_static(&installation);
    let live = extraction::collect_live(&observed, &static_answers.discovered(), unused).await;

    extraction::extraction(static_answers, live)
}

fn gap_reason(snapshot: &snapshot::Snapshot, id: &str) -> String {
    snapshot
        .gaps
        .iter()
        .find(|gap| gap.id == id)
        .unwrap_or_else(|| panic!("no gap {id}"))
        .reason
        .clone()
}

#[tokio::test]
async fn static_mode_of_one_recording_gives_the_snapshot_of_collect() {
    let native = Native::from_recorded_answers(recording()).unwrap();
    let collected = extraction::collect(&native, unused).await;
    let joined = static_mode(&recording()).await.unwrap();

    assert!(joined.complete());
    assert_eq!(
        snapshot::json_bytes(&snapshot::assemble(&collected).unwrap()).unwrap(),
        snapshot::json_bytes(&snapshot::assemble(&joined).unwrap()).unwrap()
    );
}

#[tokio::test]
async fn observations_of_another_build_are_refused() {
    let (_root, answers) = editable_recording();
    fs::write(answers.join("build.json"), b"\"another-build\"\n").unwrap();

    let error = static_mode(&answers).await.unwrap_err();

    assert!(error.contains("another-build"), "{error}");
}

#[tokio::test]
async fn a_registry_missing_from_the_recording_is_still_observed_and_its_answer_is_a_gap() {
    let (_root, answers) = editable_recording();
    let path = answers.join("registries.json");
    let mut registries: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    registries["Ok"]["value"]
        .as_array_mut()
        .unwrap()
        .retain(|registry| registry["name"] != extraction::TRADITIONS);
    fs::write(&path, serde_json::to_vec(&registries).unwrap()).unwrap();
    fs::remove_dir_all(answers.join("observe_fixture")).unwrap();

    let joined = static_mode(&answers).await.unwrap();
    let snapshot = snapshot::assemble(&joined).unwrap();

    assert!(!joined.complete());
    assert!(
        joined
            .sessions
            .iter()
            .any(|session| session.registry == extraction::TRADITIONS
                && matches!(session.observation, Err(Error::NotRecorded { .. })))
    );
    assert!(
        gap_reason(
            &snapshot,
            "registry:common/traditions#fixture.tradition_outcomes"
        )
        .starts_with("Native fixture question failed")
    );
}

#[tokio::test]
async fn a_missing_loaded_modifier_answer_is_a_gap() {
    let (_root, answers) = editable_recording();
    fs::remove_file(answers.join("loaded_modifiers.json")).unwrap();

    let joined = static_mode(&answers).await.unwrap();
    let snapshot = snapshot::assemble(&joined).unwrap();

    assert!(!joined.complete());
    assert!(matches!(
        joined.loaded_modifiers.observation,
        Err(Error::NotRecorded { .. })
    ));
    assert!(gap_reason(&snapshot, "inventory:loaded_modifiers#answer").contains("recorded"));
}

#[tokio::test]
async fn a_recorded_disposal_failure_is_carried_into_the_snapshot() {
    let (_root, answers) = editable_recording();
    let tradition_answer = fs::read_dir(answers.join("observe_fixture"))
        .unwrap()
        .flat_map(|directory| fs::read_dir(directory.unwrap().path()).unwrap())
        .map(|file| file.unwrap().path())
        .find(|file| {
            fs::read_to_string(file)
                .unwrap()
                .contains("common/traditions/atlas_frozen.txt")
        })
        .unwrap();
    let failure = Error::Cleanup {
        reason: "worker lost".into(),
        disposal: Disposal::Unconfirmed("no supervisor report".into()),
    };
    fs::write(
        &tradition_answer,
        serde_json::to_vec(&serde_json::json!({ "Err": failure })).unwrap(),
    )
    .unwrap();

    let joined = static_mode(&answers).await.unwrap();
    let snapshot = snapshot::assemble(&joined).unwrap();

    assert!(!joined.complete());
    assert!(
        joined
            .sessions
            .iter()
            .any(|session| session.observation.as_ref().err() == Some(&failure))
    );
    assert!(
        gap_reason(
            &snapshot,
            "registry:common/traditions#fixture.tradition_outcomes"
        )
        .contains("no supervisor report")
    );
}

#[tokio::test]
async fn a_static_mode_snapshot_holds_no_live_observation() {
    let joined = static_mode(&recording()).await.unwrap();
    let mut snapshot = snapshot::assemble(&joined).unwrap();

    assert!(!snapshot.observed_live());

    let source = snapshot.sources.values_mut().next().unwrap();
    source.basis = pdx_native::Basis::LiveObservation;

    assert!(snapshot.observed_live());
}
