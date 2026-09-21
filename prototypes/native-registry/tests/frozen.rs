use atlas_native_consumer::frozen::{Outcome, run};
use pdx_native::{
    Answer, Basis, Completeness, Disposal, Error, Field, FixtureStorage, GameOptions, Native,
    Operation,
};
use serde_json::{Value, json};
use std::{path::Path, process::Command};

fn write_answer(root: &Path, path: &str, value: Value) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        serde_json::to_vec(&json!({"Ok": {
            "value": value,
            "completeness": "Complete",
            "gaps": [],
            "source": {
                "build": "authored",
                "native_version": "test",
                "method": "authored",
                "basis": "LiveObservation"
            }
        }}))
        .unwrap(),
    )
    .unwrap();
}

fn row<'a>(report: &'a atlas_native_consumer::frozen::Report, id: &str) -> &'a Outcome {
    &report
        .coverage
        .iter()
        .find(|row| row.question == id)
        .unwrap()
        .outcome
}

fn authored_answers(root: &Path) {
    std::fs::write(root.join("build.json"), "\"authored\"").unwrap();
    write_answer(
        root,
        "registries.json",
        json!([
            {"name": "common/traditions"},
            {"name": "common/tradition_categories"}
        ]),
    );
    write_answer(
        root,
        "registry_fields/common/traditions.json",
        json!([
            {"name":"unlocks_agenda","reader":{"id":"shared","kind":"String"},"conditional":false},
            {"name":"custom_tooltip","reader":{"id":"shared","kind":"String"},"conditional":false},
            {"name":"tradition_swap","reader":{"id":null,"kind":"Unknown"},"conditional":false}
        ]),
    );
    write_answer(
        root,
        "registry_fields/common/tradition_categories.json",
        json!([]),
    );
    write_answer(
        root,
        "registry_items/common/tradition_categories.json",
        json!(["atlas_category"]),
    );
}

fn recording() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("recorded/m45")
}

fn copy_recording(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir(&destination).unwrap();
            copy_recording(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[tokio::test]
async fn retained_m45_answers_cover_every_requested_field() {
    let native = Native::from_recorded_answers(recording()).unwrap();
    let report = run(&native, || GameOptions::new(Command::new("/no-supervisor"))).await;
    assert_eq!(report.summary.unanswered, 0);
    assert_eq!(
        report.summary.observed + report.summary.gaps,
        report.coverage.len()
    );
    assert!(
        report
            .answers
            .sessions
            .iter()
            .all(|session| { session.termination == Ok(Disposal::NotApplicable) })
    );
    assert!(matches!(
        row(&report, "tradition.tooltip.shared_reader"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.agenda.storage.omitted"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.agenda.storage.repeated"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.parser.malformed"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.parser.unknown_field"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "category.tree_template.storage.valid"),
        Outcome::Gap {
            owner: "SDK-541",
            ..
        }
    ));
    assert!(matches!(
        row(&report, "category.parser.coverage"),
        Outcome::Gap {
            owner: "SDK-541",
            ..
        }
    ));
    assert!(matches!(
        row(&report, "tree_template.registry"),
        Outcome::Gap {
            owner: "SDK-551",
            ..
        }
    ));
    let tradition = report
        .answers
        .sessions
        .iter()
        .find(|session| session.name == "tradition_outcomes")
        .unwrap()
        .fixture
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(tradition.source.basis, Basis::Recorded);
    let omitted = tradition
        .value
        .field_outcomes
        .iter()
        .find(|outcome| {
            outcome.question.definition == "atlas_omitted"
                && outcome.question.field == "unlocks_agenda"
        })
        .unwrap();
    assert!(matches!(&omitted.storage, FixtureStorage::String {
        final_value: Some(value),
        completeness: Completeness::Complete,
        ..
    } if value.is_empty()));
    let repeated = tradition
        .value
        .field_outcomes
        .iter()
        .find(|outcome| {
            outcome.question.definition == "atlas_repeated"
                && outcome.question.field == "unlocks_agenda"
        })
        .unwrap();
    assert!(matches!(&repeated.storage, FixtureStorage::String {
        occurrences,
        completeness: Completeness::Complete,
        ..
    } if occurrences.len() == 2));
    let category = report
        .answers
        .sessions
        .iter()
        .find(|session| session.name == "category_outcomes")
        .unwrap()
        .fixture
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert!(
        category
            .value
            .field_outcomes
            .iter()
            .all(|outcome| { matches!(outcome.storage, FixtureStorage::Unavailable(_)) })
    );
}

#[tokio::test]
async fn removing_one_retained_answer_blocks_only_its_question() {
    let root = tempfile::tempdir().unwrap();
    copy_recording(&recording(), root.path());
    std::fs::remove_file(root.path().join("registry_items/common/traditions.json")).unwrap();
    let native = Native::from_recorded_answers(root.path()).unwrap();
    let report = run(&native, || GameOptions::new(Command::new("/no-supervisor"))).await;
    assert_eq!(report.summary.unanswered, 1);
    assert!(matches!(
        row(&report, "tradition.items"),
        Outcome::Unanswered {
            error: Error::NotRecorded { .. }
        }
    ));
    assert!(matches!(
        row(&report, "category.items"),
        Outcome::Observed { .. }
    ));
}

#[tokio::test]
async fn authored_field_error_is_preserved_for_dependent_questions() {
    let root = tempfile::tempdir().unwrap();
    copy_recording(&recording(), root.path());
    let error = Error::Observation {
        operation: Operation::RegistryFields,
        reason: "authored field failure".into(),
    };
    let result: Result<Answer<Vec<Field>>, Error> = Err(error.clone());
    std::fs::write(
        root.path()
            .join("registry_fields/common/tradition_categories.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    let native = Native::from_recorded_answers(root.path()).unwrap();
    let report = run(&native, || GameOptions::new(Command::new("/no-supervisor"))).await;
    for question in [
        "category.desc.field",
        "category.desc.reader",
        "category.tree_template.field",
    ] {
        assert!(
            matches!(row(&report, question), Outcome::Unanswered { error: actual } if *actual == error)
        );
    }
    assert!(matches!(
        row(&report, "tradition.tooltip.shared_reader"),
        Outcome::Observed { .. }
    ));
}

#[tokio::test]
async fn absent_answer_is_unanswered_without_starting_a_process() {
    let root = tempfile::tempdir().unwrap();
    authored_answers(root.path());
    let native = pdx_native::Native::from_recorded_answers(root.path()).unwrap();
    let report = run(&native, || GameOptions::new(Command::new("/no-supervisor"))).await;

    assert!(matches!(
        row(&report, "tradition.items"),
        Outcome::Unanswered {
            error: Error::NotRecorded { .. }
        }
    ));
    assert!(matches!(
        row(&report, "category.items"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.tooltip.shared_reader"),
        Outcome::Observed { .. }
    ));
    assert!(matches!(
        row(&report, "tradition.tradition_swap.reader"),
        Outcome::Gap {
            owner: "SDK-541",
            ..
        }
    ));
    assert!(matches!(
        row(&report, "tradition.icon"),
        Outcome::Gap {
            owner: "SDK-546",
            ..
        }
    ));
    assert!(report.summary.unanswered > 0);
    assert!(
        report
            .answers
            .sessions
            .iter()
            .all(|session| { session.termination == Ok(Disposal::NotApplicable) })
    );
}

#[test]
fn recorded_cli_writes_report_and_exits_nonzero_for_an_absent_answer() {
    let root = tempfile::tempdir().unwrap();
    authored_answers(root.path());
    let output = Command::new(env!("CARGO_BIN_EXE_atlas-native-consumer"))
        .arg("frozen-recorded")
        .arg(root.path())
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["summary"]["unanswered"].as_u64().unwrap() > 0);
    assert_eq!(
        report["answers"]["sessions"][0]["termination"]["Ok"],
        "NotApplicable"
    );
}
