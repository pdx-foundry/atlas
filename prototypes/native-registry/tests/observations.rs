use atlas_native_consumer::{FixtureKey, REGISTRIES, collect, process_observation};
use pdx_native::{Answer, Basis, Completeness, Disposal, Error, GameOptions, Native};
use std::{path::Path, process::Command};

fn answer(partial: bool) -> Answer<Vec<String>> {
    serde_json::from_value(serde_json::json!({
        "value": ["atlas_early_category"],
        "completeness": if partial { "Partial" } else { "Complete" },
        "gaps": if partial { serde_json::json!([{"kind":"IncompleteObservation", "subject": "common/tradition_categories", "detail":"missing terminal"}]) } else { serde_json::json!([]) },
        "source": {"build":"authored", "native_version":"test", "method":"registry-items/v1", "basis":"LiveObservation"}
    })).unwrap()
}

fn write(root: &Path, registry: &str, result: Result<Answer<Vec<String>>, Error>) {
    let path = root.join("registry_items").join(format!("{registry}.json"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(root.join("build.json"), "\"authored\"").unwrap();
    std::fs::write(path, serde_json::to_vec(&result).unwrap()).unwrap();
}

#[test]
fn processor_preserves_values_partial_gaps_and_source() {
    for partial in [false, true] {
        let original = answer(partial);
        let observation = process_observation(REGISTRIES[1], original.clone());
        assert_eq!(observation.native, original);
        assert_eq!(observation.fixture_key, FixtureKey::Observed);
        assert_eq!(
            process_observation(REGISTRIES[0], original).fixture_key,
            FixtureKey::Unknown
        );
    }
    let mut missing = answer(true);
    missing.value.clear();
    let observation = process_observation(REGISTRIES[1], missing);
    assert_eq!(observation.fixture_key, FixtureKey::NotObservedInAnswer);
    assert_eq!(observation.native.completeness, Completeness::Partial);
}

#[tokio::test]
async fn recorded_failures_keep_the_other_answer_and_still_close() {
    for failure in [
        Error::Observation {
            operation: pdx_native::Operation::RegistryItems,
            reason: "worker lost".into(),
        },
        Error::Startup {
            reason: "timeout".into(),
            disposal: Disposal::Confirmed,
        },
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), REGISTRIES[0], Err(failure.clone()));
        write(root.path(), REGISTRIES[1], Ok(answer(true)));
        let native = Native::from_recorded_answers(root.path()).unwrap();
        let output = collect(
            &native,
            GameOptions::new(Command::new("/nonexistent-supervisor")),
        )
        .await;
        assert_eq!(
            output.queries[REGISTRIES[0]].as_ref().unwrap_err(),
            &failure
        );
        let category = output.queries[REGISTRIES[1]].as_ref().unwrap();
        assert_eq!(category.native.source.basis, Basis::Recorded);
        assert_eq!(category.native.completeness, Completeness::Partial);
        assert_eq!(output.termination, Ok(Disposal::NotApplicable));
    }
}

#[tokio::test]
async fn missing_and_corrupt_answers_are_not_empty_successes() {
    for corrupt in [false, true] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), REGISTRIES[1], Ok(answer(false)));
        if corrupt {
            std::fs::write(
                root.path().join("registry_items/common/traditions.json"),
                "bad json",
            )
            .unwrap();
        }
        let native = Native::from_recorded_answers(root.path()).unwrap();
        let output = collect(
            &native,
            GameOptions::new(Command::new("/nonexistent-supervisor")),
        )
        .await;
        assert!(matches!(
            output.queries[REGISTRIES[0]],
            Err(Error::Recorded(_) | Error::NotRecorded { .. })
        ));
        assert!(output.queries[REGISTRIES[1]].is_ok());
        assert_eq!(output.termination, Ok(Disposal::NotApplicable));
    }
}

#[test]
fn recorded_cli_needs_no_installation_and_reports_each_failure() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), REGISTRIES[1], Ok(answer(false)));
    let output = Command::new(env!("CARGO_BIN_EXE_atlas-native-consumer"))
        .arg("recorded")
        .arg(root.path())
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["queries"][REGISTRIES[0]]["Err"]["NotRecorded"].is_object());
    assert_eq!(
        value["queries"][REGISTRIES[1]]["Ok"]["native"]["source"]["basis"],
        "Recorded"
    );
    assert_eq!(value["termination"]["Ok"], "NotApplicable");
}
