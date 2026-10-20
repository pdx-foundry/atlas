//! Content-derived documentation credited by the shipped comments of the snapshot's build.
use pdx_atlas::{coverage, extraction, ledger, report, snapshot};
use pdx_native::{Basis, GameOptions, Native};
use std::{collections::BTreeMap, path::Path, process::Command};

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(file, text)| ((*file).into(), (*text).into()))
        .collect()
}

async fn recorded_snapshot() -> snapshot::Snapshot {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/m452");
    let native = Native::from_recorded_answers(path).unwrap();
    let extraction =
        extraction::collect(&native, || GameOptions::new(Command::new("unused"))).await;

    snapshot::assemble(&extraction).unwrap()
}

fn live_snapshot(mut snapshot: snapshot::Snapshot) -> snapshot::Snapshot {
    let mut keys = BTreeMap::new();

    for (old, mut source) in std::mem::take(&mut snapshot.sources) {
        source.basis = Basis::LiveObservation;
        let new = format!("{}@live_observation", source.method);
        keys.insert(old, new.clone());
        snapshot.sources.insert(new, source);
    }

    for answer in snapshot.answers.values_mut() {
        answer.source = keys[&answer.source].clone();
    }

    snapshot
}

fn build(snapshot: &snapshot::Snapshot) -> String {
    serde_json::to_value(&snapshot.applicability.builds[0])
        .unwrap()
        .as_str()
        .unwrap()
        .into()
}

/// The coverage of `config` against the live M452 snapshot and the shipped files `content`.
struct Run {
    ledger: ledger::Ledger,
    report: coverage::Report,
}

impl Run {
    async fn live(config: &[(&str, &str)], content: &[(&str, &str)]) -> Self {
        let snapshot = live_snapshot(recorded_snapshot().await);

        Self::with(config, content, &snapshot)
    }

    fn with(
        config: &[(&str, &str)],
        content: &[(&str, &str)],
        snapshot: &snapshot::Snapshot,
    ) -> Self {
        let ledger = ledger::inventory(&files(config));
        let content = coverage::Content::new(build(snapshot), &files(content));
        let bytes = snapshot::json_bytes(snapshot).unwrap();
        let report = coverage::evaluate(&ledger, Some(&bytes), Some(&content)).unwrap();

        Self { ledger, report }
    }

    /// The assessments of the documentation claims, in source order.
    fn documentation(&self) -> Vec<&coverage::Assessment> {
        let mut claims: Vec<_> = self
            .ledger
            .claims
            .iter()
            .filter(|c| c.property == "documentation")
            .collect();

        claims.sort_by_key(|c| c.span.start);
        claims
            .iter()
            .map(|claim| {
                self.report
                    .claims
                    .iter()
                    .find(|a| a.claim == claim.id)
                    .unwrap()
            })
            .collect()
    }

    fn gap_reasons(&self) -> Vec<&str> {
        self.report
            .content
            .as_ref()
            .unwrap()
            .gaps
            .iter()
            .map(|gap| gap.reason.as_str())
            .collect()
    }
}

const THINGS: &str = "types = { type[thing] = { path = \"game/common/things\" } }\n";

#[tokio::test]
async fn an_exact_shipped_comment_credits_content_derived_documentation() {
    let config = format!("{THINGS}thing = {{\n ### A documented\n #### field\n value = int\n}}\n");
    let run = Run::live(
        &[("common/things.cwt", &config)],
        &[(
            "common/things/00.txt",
            "example = {\n value = 2 # A documented field\n}\n",
        )],
    )
    .await;
    let [documentation] = run.documentation()[..] else {
        panic!("one documentation claim");
    };

    assert!(documentation.covered, "{}", documentation.reason);
    assert_eq!(
        documentation.evidence,
        ["shipped-comment:common/things/00.txt:2-2"]
    );

    let content = run.report.content.as_ref().unwrap();

    assert_eq!(content.answered, 1);
    assert!(content.gaps.is_empty());
    assert_eq!(content.files, 1);
}

#[tokio::test]
async fn a_shipped_comment_never_credits_an_engine_fact() {
    let run = Run::live(
        &[(
            "effects.cwt",
            "### Changes the amount of energy\nalias[effect:add_energy] = int\n",
        )],
        &[(
            "common/effects/00.txt",
            "# Changes the amount of energy\nadd_energy = 1\n",
        )],
    )
    .await;
    let claim = run
        .ledger
        .claims
        .iter()
        .find(|c| c.property == "documentation")
        .unwrap();

    assert_eq!(claim.owner, ledger::Owner::EngineFact);
    assert!(!run.documentation()[0].covered);

    let content = run.report.content.as_ref().unwrap();

    assert_eq!(content.answered, 0);
    assert!(content.gaps.is_empty());
}

#[tokio::test]
async fn a_rewritten_candidate_keeps_a_gap_with_its_location() {
    let config = format!(
        "{THINGS}thing = {{\n ### Changes the total amount of energy stored by the country\n value = int\n}}\n"
    );
    let run = Run::live(
        &[("common/things.cwt", &config)],
        &[(
            "common/things/00.txt",
            "example = {\n # Changes the amount of energy stored by the country\n value = 2\n}\n",
        )],
    )
    .await;

    assert!(!run.documentation()[0].covered);
    assert_eq!(
        run.gap_reasons(),
        [
            "The shipped comment at common/things/00.txt:2-2 is a rewritten candidate, not the same text"
        ]
    );
}

#[tokio::test]
async fn a_recorded_snapshot_earns_no_content_credit() {
    let config = format!("{THINGS}thing = {{\n ### Documented\n value = int\n}}\n");
    let run = Run::with(
        &[("common/things.cwt", &config)],
        &[(
            "common/things/00.txt",
            "example = {\n value = 2 # Documented\n}\n",
        )],
        &recorded_snapshot().await,
    );

    assert!(!run.documentation()[0].covered);
    assert_eq!(run.report.content.as_ref().unwrap().answered, 1);
}

#[tokio::test]
async fn content_needs_a_snapshot_of_its_build() {
    let ledger = ledger::inventory(&files(&[("common/things.cwt", THINGS)]));
    let snapshot = live_snapshot(recorded_snapshot().await);
    let bytes = snapshot::json_bytes(&snapshot).unwrap();
    let shipped = files(&[("common/things/00.txt", "a = 1\n")]);
    let other = coverage::Content::new("another build", &shipped);
    let same = coverage::Content::new(build(&snapshot), &shipped);

    assert!(coverage::evaluate(&ledger, Some(&bytes), Some(&other)).is_err());
    assert!(coverage::evaluate(&ledger, None, Some(&same)).is_err());
}

#[tokio::test]
async fn matching_alone_changes_neither_ownership_nor_the_denominator() {
    let config = format!(
        "{THINGS}thing = {{\n ### Documented\n value = int\n ### Not shipped\n other = int\n}}\n"
    );
    let content = [(
        "common/things/00.txt",
        "example = {\n value = 2 # Documented\n}\n",
    )];
    let with_content = Run::live(&[("common/things.cwt", &config)], &content).await;
    let without = Run::live(&[("common/things.cwt", &config)], &[]).await;

    assert_eq!(
        with_content.report.totals.atlas_owned.total,
        without.report.totals.atlas_owned.total
    );
    assert_eq!(
        with_content.gap_reasons(),
        ["No shipped comment in the installation has this text"]
    );
}

#[tokio::test]
async fn documentation_that_no_shipped_comment_holds_is_authored_text() {
    let config = format!(
        "{THINGS}thing = {{
 ### Documented
 exact = int
 ### Not shipped
 absent = int
 ### Changes the total amount of energy that is stored by the country
 held = int
 ### Changes the total amount of energy stored by the country
 reworded = int
}}
"
    );
    let shipped = files(&[(
        "common/things/00.txt",
        "example = {
 exact = 2 # Documented
 held = 2 # changes the total amount of energy that is stored by the country Optional
 # Changes the amount of energy stored by the country
 reworded = 2
}
",
    )]);
    let snapshot = live_snapshot(recorded_snapshot().await);
    let mut ledger = ledger::inventory(&files(&[("common/things.cwt", &config)]));
    let before = coverage::evaluate(&ledger, None, None)
        .unwrap()
        .totals
        .atlas_owned
        .total;

    coverage::attribute_authored_text(
        &mut ledger,
        &coverage::Content::new(build(&snapshot), &shipped),
    );

    let owner = |key: &str| {
        ledger
            .claims
            .iter()
            .find(|c| c.property == "documentation" && c.subject.last().unwrap() == key)
            .unwrap()
            .owner
    };
    let after = coverage::evaluate(&ledger, None, None)
        .unwrap()
        .totals
        .atlas_owned
        .total;

    assert_eq!(owner("exact"), ledger::Owner::ContentDerived);
    assert_eq!(owner("held"), ledger::Owner::ContentDerived);
    assert_eq!(owner("absent"), ledger::Owner::AuthoredText);
    assert_eq!(owner("reworded"), ledger::Owner::AuthoredText);
    assert_eq!(after, before - 2);
}

#[tokio::test]
async fn every_occurrence_of_a_question_must_match_exactly() {
    let content = [(
        "common/things/00.txt",
        "a = {\n field = 1 # First text\n}\nb = {\n # Changes the amount of energy stored by the country\n field = 2\n}\n",
    )];
    let cases = [
        ("First text", "First text", true),
        (
            "First text",
            "Changes the amount of energy stored by the country",
            true,
        ),
        (
            "First text",
            "Changes the total amount of energy stored by the country",
            false,
        ),
        ("First text", "Not shipped", false),
    ];

    for (first, second, covered) in cases {
        let config = format!(
            "{THINGS}thing = {{\n ### {first}\n field = int\n ### {second}\n field = bool\n}}\n"
        );
        let run = Run::live(&[("common/things.cwt", &config)], &content).await;
        let documentation = run.documentation();
        let question: Vec<_> = run
            .ledger
            .claims
            .iter()
            .filter(|c| c.property == "documentation")
            .map(|c| &c.question)
            .collect();

        assert_eq!(question[0], question[1], "one question");
        assert!(
            documentation.iter().all(|a| a.covered == covered),
            "{first} / {second}"
        );
    }
}

#[tokio::test]
async fn type_folders_include_descendants_but_not_sibling_prefixes() {
    let config = "types = { type[thing] = { path = \"common/things\" } }\nthing = {\n ### Nested documentation\n field = int\n ### Sibling documentation\n other = int\n}\n";
    let run = Run::live(
        &[("common/schema.cwt", config)],
        &[
            (
                "common/things/guides/example.txt",
                "# field: Nested documentation\n",
            ),
            (
                "common/things_elsewhere/guide.txt",
                "# other: Sibling documentation\n",
            ),
        ],
    )
    .await;

    assert_eq!(
        run.documentation()
            .iter()
            .map(|a| a.covered)
            .collect::<Vec<_>>(),
        [true, false]
    );
}

#[tokio::test]
async fn a_claim_reads_only_its_own_types_folder() {
    let config = "types = {\n type[a] = { path = \"game/common/a\" }\n type[b] = { path = \"game/common/b\" }\n}\na = {\n ### Shared words\n field = int\n}\nb = {\n ### Shared words\n field = int\n}\n";
    let run = Run::live(
        &[("common/ab.cwt", config)],
        &[("common/b/00.txt", "x = {\n field = 1 # Shared words\n}\n")],
    )
    .await;

    assert_eq!(
        run.documentation()
            .iter()
            .map(|a| a.covered)
            .collect::<Vec<_>>(),
        [false, true]
    );
}

#[tokio::test]
async fn a_define_comment_must_sit_under_its_namespace() {
    let config = "defines = {\n Right = {\n ### shared words\n VALUE = int\n }\n Wrong = {\n ### shared words\n VALUE = int\n }\n}\n";
    let run = Run::live(
        &[("common/defines/00_defines.cwt", config)],
        &[(
            "common/defines/00.txt",
            "Right = {\n VALUE = 1 # shared words\n}\n",
        )],
    )
    .await;

    assert_eq!(
        run.documentation()
            .iter()
            .map(|a| a.covered)
            .collect::<Vec<_>>(),
        [true, false]
    );
}

#[tokio::test]
async fn an_on_action_is_documented_under_its_name() {
    let run = Run::live(
        &[(
            "on_actions.cwt",
            "on_actions = {\n ### Fires when a game begins\n on_game_start\n}\n",
        )],
        &[(
            "common/on_actions/00.txt",
            "# Fires when a game begins\non_game_start = {}\n",
        )],
    )
    .await;

    assert!(run.documentation()[0].covered);
}

#[tokio::test]
async fn the_content_digest_follows_the_shipped_files() {
    let shipped = files(&[("common/things/00.txt", "a = 1 # text\n")]);
    let mut changed = shipped.clone();
    changed.insert("common/things/00.txt".into(), "a = 1 # other\n".into());
    let ledger = ledger::inventory(&files(&[("common/things.cwt", THINGS)]));
    let snapshot = recorded_snapshot().await;
    let bytes = snapshot::json_bytes(&snapshot).unwrap();
    let digest = |files: &BTreeMap<String, String>| {
        let content = coverage::Content::new(build(&snapshot), files);

        coverage::evaluate(&ledger, Some(&bytes), Some(&content))
            .unwrap()
            .content
            .unwrap()
            .sha256
    };

    assert_eq!(digest(&shipped), digest(&shipped.clone()));
    assert_ne!(digest(&shipped), digest(&changed));
}

#[cfg(unix)]
#[test]
fn shipped_files_are_read_below_common_without_following_symlinks() {
    let installation = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let things = installation.path().join("common/things");

    std::fs::create_dir_all(&things).unwrap();
    std::fs::write(things.join("00.txt"), "a = 1 # text\n").unwrap();
    std::fs::write(things.join("notes.md"), "# not content\n").unwrap();

    assert_eq!(
        report::read_game_content(installation.path())
            .unwrap()
            .into_keys()
            .collect::<Vec<_>>(),
        ["common/things/00.txt"]
    );

    std::fs::create_dir_all(other.path().join("common")).unwrap();
    std::fs::rename(
        installation.path().join("common"),
        other.path().join("moved"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        other.path().join("moved"),
        installation.path().join("common"),
    )
    .unwrap();

    assert!(report::read_game_content(installation.path()).is_err());
}
