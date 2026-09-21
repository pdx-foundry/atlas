use atlas_native_consumer::frozen::{Outcome, comparable, run};
use pdx_native::{Disposal, GameOptions, Native};
use std::process::Command;

#[tokio::test]
#[ignore = "requires STELLARIS_PATH with the exact supported build"]
async fn live_and_recorded_frozen_questions_have_the_same_answers() {
    let installation = std::env::var_os("STELLARIS_PATH").expect("STELLARIS_PATH is required");
    let recorded = tempfile::tempdir().unwrap();
    let live = Native::open(installation)
        .unwrap()
        .record_answers_to(recorded.path());
    let supervisor = env!("CARGO_BIN_EXE_atlas-native-consumer");
    let live_report = run(&live, || {
        let mut command = Command::new(supervisor);
        command.arg("--supervisor");
        GameOptions::new(command)
    })
    .await;
    let unanswered: Vec<_> = live_report
        .coverage
        .iter()
        .filter(|row| matches!(row.outcome, Outcome::Unanswered { .. }))
        .map(|row| (&row.question, &row.outcome))
        .collect();
    assert!(unanswered.is_empty(), "{unanswered:?}");
    assert!(
        live_report
            .answers
            .sessions
            .iter()
            .all(|session| { session.termination == Ok(Disposal::Confirmed) }),
        "{:#?}",
        live_report.answers.sessions
    );

    let replay = Native::from_recorded_answers(recorded.path()).unwrap();
    let replay_report = run(&replay, || GameOptions::new(Command::new("/no-supervisor"))).await;
    let unanswered: Vec<_> = replay_report
        .coverage
        .iter()
        .filter(|row| matches!(row.outcome, Outcome::Unanswered { .. }))
        .map(|row| (&row.question, &row.outcome))
        .collect();
    assert!(unanswered.is_empty(), "{unanswered:?}");
    assert!(
        replay_report
            .answers
            .sessions
            .iter()
            .all(|session| { session.termination == Ok(Disposal::NotApplicable) })
    );
    assert_eq!(comparable(&live_report), comparable(&replay_report));
    assert!(live_report.coverage.iter().any(|row| {
        row.question == "tradition.tooltip.shared_reader"
            && matches!(row.outcome, Outcome::Observed { .. })
    }));
    assert!(live_report.coverage.iter().any(|row| {
        row.question == "tree_template.registry"
            && matches!(
                row.outcome,
                Outcome::Gap {
                    owner: "SDK-551",
                    ..
                }
            )
    }));
}
