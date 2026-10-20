use pdx_atlas::{
    coverage::{self, comparison},
    dashboard::Baseline,
    extraction, report, snapshot,
};
use pdx_native::{Completeness, GameOptions, Native, supervisor};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

/// The revision whose committed baseline each ledger run is compared with by default.
const BASELINE_REVISION: &str = "origin/main";
/// Where a revision keeps its baseline.
const BASELINE_PATH: &str = "docs/coverage/covered/";

fn run_ledger() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    args.next();
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if ![
            "--config",
            "--snapshot",
            "--game-content",
            "--output",
            "--dashboard",
            "--baseline",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("Unknown argument: {key}").into());
        }
        let value = args.next().ok_or("Missing argument value")?;
        if value.starts_with("--") {
            return Err(format!("Missing value for {key}").into());
        }
        if options.insert(key.clone(), PathBuf::from(value)).is_some() {
            return Err(format!("Duplicate argument: {key}").into());
        }
    }
    let config = options.get("--config").ok_or("--config is required")?;
    let output = options.get("--output").ok_or("--output is required")?;
    let snapshot = options.get("--snapshot").map(PathBuf::as_path);
    let observed_live = match snapshot {
        Some(path) => serde_json::from_slice::<snapshot::Snapshot>(&std::fs::read(path)?)
            .map_err(|error| format!("Invalid rule snapshot input: {error}"))?
            .observed_live(),
        None => false,
    };
    let baseline = match options.get("--baseline") {
        Some(directory) => Some(named_baseline(directory, observed_live)?),
        None => committed_baseline(observed_live)?,
    };
    let content = options
        .get("--game-content")
        .map(|installation| game_content(installation))
        .transpose()?;
    let (ledger, coverage) = report::generate(
        config,
        snapshot,
        content.as_ref(),
        observed_live,
        output,
        baseline.as_ref(),
    )?;
    let headline = &coverage.totals.atlas_owned;
    let percentage = headline
        .percent
        .map_or("not applicable".into(), |n| format!("{n:.6}%"));
    println!(
        "{} CWT files; {} claims; Atlas-owned coverage: {}/{} ({percentage})",
        ledger.files.len(),
        ledger.claims.len(),
        headline.covered,
        headline.total
    );
    println!("Reports: {}", output.display());
    let dashboard = options
        .get("--dashboard")
        .cloned()
        .or_else(shared_dashboard);
    match dashboard.map(|directory| copy_dashboard(output, &directory)) {
        Some(Ok(page)) => println!("Dashboard: {}", page.display()),
        Some(Err(error)) => eprintln!("The dashboard was not copied: {error}"),
        None => eprintln!("No shared dashboard folder; the page is only in the reports folder."),
    }
    if !ledger.diagnostics.is_empty() {
        eprintln!(
            "{} source diagnostics remain; percentages cover inventoried claims only. See ledger.json.",
            ledger.diagnostics.len()
        );
    }
    let content_complete = content.is_none_or(|content| content.diagnostics.is_empty());

    Ok(ledger.diagnostics.is_empty() && content_complete)
}

/// The shipped files of `installation`, identified by its build. Native hashes the executable;
/// no game starts.
fn game_content(installation: &Path) -> Result<coverage::Content, Box<dyn std::error::Error>> {
    let build = serde_json::to_value(Native::open(installation)?.build())?
        .as_str()
        .ok_or("Native build id is not a string")?
        .to_owned();
    let content = coverage::Content::new(build, &report::read_game_content(installation)?);

    if !content.diagnostics.is_empty() {
        eprintln!(
            "{} parse problems in the shipped files; their comments are bound by lines. See coverage.json.",
            content.diagnostics.len()
        );
    }

    Ok(content)
}

/// Runs git in the checkout that built this binary, returning its output on success.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

/// The kind of run, which a baseline must share to be compared.
fn run_kind(observed_live: bool) -> &'static str {
    if observed_live {
        "live"
    } else {
        "static or recorded"
    }
}

/// The baseline in `directory`, which must be of the same kind of run as this one.
fn named_baseline(
    directory: &Path,
    observed_live: bool,
) -> Result<Baseline, Box<dyn std::error::Error>> {
    let baseline = report::read_baseline(directory)?;

    if baseline.observed_live != observed_live {
        return Err(format!(
            "The baseline in {} is of a {} run, and this run is {}; they do not compare.",
            directory.display(),
            run_kind(baseline.observed_live),
            run_kind(observed_live)
        )
        .into());
    }

    Ok(baseline)
}

/// The committed baseline of `main` when it is of the same kind of run as this one. The committed
/// baseline is a live capture, so a static-mode run is not compared with it.
fn committed_baseline(observed_live: bool) -> Result<Option<Baseline>, String> {
    let Some(baseline) = main_baseline()? else {
        return Ok(None);
    };

    if baseline.observed_live != observed_live {
        eprintln!(
            "The baseline of {BASELINE_REVISION} is of a {} run, and this run is {}: the dashboard does not compare them. Pass --baseline DIR to compare with another run.",
            run_kind(baseline.observed_live),
            run_kind(observed_live)
        );
        return Ok(None);
    }

    Ok(Some(baseline))
}

/// The committed baseline of `main`, or `None` with a warning when git cannot show it.
fn main_baseline() -> Result<Option<Baseline>, String> {
    let listing = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASELINE_REVISION,
        "--",
        BASELINE_PATH,
    ])
    .filter(|listing| !listing.trim().is_empty());
    let Some(listing) = listing else {
        eprintln!(
            "No baseline at {BASELINE_REVISION}:{BASELINE_PATH}; the dashboard does not measure the change."
        );
        return Ok(None);
    };
    let mut texts = BTreeMap::new();

    for path in listing.lines().filter(|path| path.ends_with(".txt")) {
        let revision_path = format!("{BASELINE_REVISION}:{path}");
        let text =
            git(&["show", &revision_path]).ok_or(format!("git cannot show {revision_path}"))?;
        let relative = path.strip_prefix(BASELINE_PATH).unwrap_or(path);

        texts.insert(relative.to_owned(), text);
    }

    Baseline::parse(&texts).map(Some)
}

/// `.local/dashboard` of the main checkout, shared by every worktree of the repository.
fn shared_dashboard() -> Option<PathBuf> {
    let common = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;

    Some(Path::new(common.trim()).parent()?.join(".local/dashboard"))
}

/// Copies the reports folder's page to `directory`, unless it is already that file.
fn copy_dashboard(output: &Path, directory: &Path) -> io::Result<PathBuf> {
    let page = output.join("dashboard.html");
    let copy = directory.join("dashboard.html");
    std::fs::create_dir_all(directory)?;

    if std::fs::canonicalize(&page)? != std::fs::canonicalize(directory)?.join("dashboard.html") {
        std::fs::copy(&page, &copy)?;
    }

    Ok(copy)
}

const COMPARE_USAGE: &str = "usage: pdx-atlas compare CONFIG_DIR SNAPSHOT_FILE OUTPUT_DIR [--script-docs DIR] [--defines FILE]... [--answers DIR]";

fn run_compare() -> Result<bool, Box<dyn std::error::Error>> {
    let mut positional = Vec::new();
    let mut script_docs = None;
    let mut defines = Vec::new();
    let mut answers = None;
    let mut args = std::env::args_os().skip(2);
    while let Some(arg) = args.next() {
        let mut value = || args.next().map(PathBuf::from).ok_or(COMPARE_USAGE);
        match arg.to_str() {
            Some("--script-docs") if script_docs.is_none() => script_docs = Some(value()?),
            Some("--defines") => defines.push(value()?),
            Some("--answers") if answers.is_none() => answers = Some(value()?),
            Some(flag) if flag.starts_with("--") => return Err(COMPARE_USAGE.into()),
            _ => positional.push(PathBuf::from(arg)),
        }
    }
    let [config, snapshot, output] = positional.as_slice() else {
        return Err(COMPARE_USAGE.into());
    };
    let loaded = match answers {
        Some(answers) => {
            let native = Native::from_recorded_answers(answers)?;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()?;
            let session = runtime.block_on(extraction::read_loaded_modifiers(
                &native,
                GameOptions::new(Command::new("unused")),
            ));
            Some(session.observation?)
        }
        None => None,
    };
    let inputs = comparison::Inputs {
        script_docs: script_docs
            .map(|directory| report::read_script_docs(&directory))
            .transpose()?
            .unwrap_or_default(),
        define_files: report::read_define_files(&defines)?,
        loaded_modifiers: loaded.as_ref(),
    };
    let report = report::generate_comparison(config, snapshot, output, &inputs)?;
    println!(
        "{} comparison entries; report: {}",
        report.entries.len(),
        output.join("comparison.json").display()
    );
    Ok(report.inventory_complete)
}

fn run_registries() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(2).map(PathBuf::from).collect();
    let [config, answers, output] = args.as_slice() else {
        return Err("usage: pdx-atlas registries CONFIG_DIR ANSWERS OUTPUT_DIR".into());
    };
    let registries = Native::from_recorded_answers(answers)?.registries()?;
    let report = report::generate_registries(config, &registries, output)?;
    println!(
        "{} directories in both; {} engine only; types by join: {:?}; report: {}",
        report.directories_in_both,
        report.engine_only.len(),
        report.types_by_join,
        output.join("registries.json").display()
    );
    Ok(report.inventory_complete && report.native_completeness == Completeness::Complete)
}

enum SnapshotMode {
    Live {
        installation: PathBuf,
        answers: PathBuf,
        seconds: Option<u64>,
    },
    Recorded {
        answers: PathBuf,
    },
    /// Static answers from the installation, game-session answers from an earlier recording.
    Static {
        installation: PathBuf,
        observations: PathBuf,
    },
}

const STATIC_USAGE: &str =
    "usage: pdx-atlas snapshot --static INSTALLATION --observations ANSWERS OUTPUT";

fn parse_snapshot_args(
    args: &[OsString],
) -> Result<(SnapshotMode, PathBuf), Box<dyn std::error::Error>> {
    if args.first().is_some_and(|arg| arg == "--recorded") {
        if args.len() != 3 {
            return Err("usage: pdx-atlas snapshot --recorded ANSWERS OUTPUT".into());
        }
        return Ok((
            SnapshotMode::Recorded {
                answers: PathBuf::from(&args[1]),
            },
            PathBuf::from(&args[2]),
        ));
    }
    if args.first().is_some_and(|arg| arg == "--static") {
        let [_, installation, flag, observations, output] = args else {
            return Err(STATIC_USAGE.into());
        };
        if flag != "--observations" {
            return Err(STATIC_USAGE.into());
        }
        return Ok((
            SnapshotMode::Static {
                installation: PathBuf::from(installation),
                observations: PathBuf::from(observations),
            },
            PathBuf::from(output),
        ));
    }
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: pdx-atlas snapshot INSTALLATION ANSWERS OUTPUT [STARTUP_SECONDS]".into(),
        );
    }
    let seconds = match args.get(3) {
        Some(value) => Some(value.to_str().ok_or("Invalid startup seconds")?.parse()?),
        None => None,
    };
    Ok((
        SnapshotMode::Live {
            installation: PathBuf::from(&args[0]),
            answers: PathBuf::from(&args[1]),
            seconds,
        },
        PathBuf::from(&args[2]),
    ))
}

fn run_snapshot() -> Result<bool, Box<dyn std::error::Error>> {
    let (mode, output) = parse_snapshot_args(&std::env::args_os().skip(2).collect::<Vec<_>>())?;
    let mut checksum_path = output.clone().into_os_string();
    checksum_path.push(".sha256");
    let checksum_path = PathBuf::from(checksum_path);
    for path in [&checksum_path, &output] {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let (native, seconds) = match mode {
        SnapshotMode::Live {
            installation,
            answers,
            seconds,
        } => (
            Native::open(installation)?.record_answers_to(answers),
            seconds,
        ),
        SnapshotMode::Recorded { answers } => (Native::from_recorded_answers(answers)?, None),
        SnapshotMode::Static {
            installation,
            observations,
        } => {
            let extraction = runtime.block_on(static_extraction(&installation, &observations))?;
            return write_snapshot(&extraction, &output, &checksum_path);
        }
    };
    let executable = std::env::current_exe()?;
    let extraction = runtime.block_on(extraction::collect(&native, || {
        let mut host = Command::new(&executable);
        host.arg("--supervisor");
        let mut options = GameOptions::new(host);
        if let Some(seconds) = seconds {
            options.startup_seconds = seconds;
        }
        options
    }));
    write_snapshot(&extraction, &output, &checksum_path)
}

/// Fresh static answers from `installation` joined with the game-session answers recorded in
/// `observations`. No game starts: the recorded Native answers every session from its files.
async fn static_extraction(
    installation: &Path,
    observations: &Path,
) -> Result<extraction::Extraction, Box<dyn std::error::Error>> {
    let installation = Native::open(installation)?;
    let recording = Native::from_recorded_answers(observations)?;
    let static_answers = extraction::collect_static(&installation);
    let live = extraction::collect_live(&recording, &static_answers.discovered(), || {
        GameOptions::new(Command::new("unused"))
    })
    .await;

    Ok(extraction::extraction(static_answers, live)?)
}

/// Writes the snapshot of `extraction` and its digest. Returns whether the extraction is complete.
fn write_snapshot(
    extraction: &extraction::Extraction,
    output: &Path,
    checksum_path: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let snapshot = snapshot::assemble(extraction)?;
    let bytes = snapshot::json_bytes(&snapshot)?;
    std::fs::write(output, &bytes)?;
    std::fs::write(checksum_path, format!("{:x}\n", Sha256::digest(&bytes)))?;
    println!(
        "{} rules; {} gaps",
        snapshot.rules.len(),
        snapshot.gaps.len()
    );
    Ok(extraction.complete())
}

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("--supervisor") => supervisor::serve(io::stdin(), io::stdout())
            .map(|()| true)
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>),
        Some("ledger") => run_ledger(),
        Some("compare") => run_compare(),
        Some("registries") => run_registries(),
        Some("snapshot") => run_snapshot(),
        _ => Err("usage: pdx-atlas ledger --config DIR [--snapshot FILE [--game-content INSTALLATION]] --output DIR [--dashboard DIR] [--baseline DIR] | compare CONFIG_DIR SNAPSHOT_FILE OUTPUT_DIR [--script-docs DIR] [--defines FILE]... [--answers DIR] | registries CONFIG_DIR ANSWERS OUTPUT_DIR | snapshot INSTALLATION ANSWERS OUTPUT [STARTUP_SECONDS] | snapshot --recorded ANSWERS OUTPUT | snapshot --static INSTALLATION --observations ANSWERS OUTPUT".into()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
