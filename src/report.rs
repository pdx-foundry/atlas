//! Filesystem boundary for reproducible config inventories and coverage reports.
use crate::{
    coverage::{self, comparison},
    dashboard::{self, Baseline},
    ledger::{self, Ledger, Sources},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

/// Reads every file with `extension` below `dir`, keyed by its path relative to `root`.
fn visit(root: &Path, dir: &Path, extension: &str, sources: &mut Sources) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Symlink input is not supported: {}", path.display()),
            ));
        }
        if kind.is_dir() {
            visit(root, &path, extension, sources)?;
        } else if path.extension().is_some_and(|s| s == extension) {
            let relative = path.strip_prefix(root).expect("descendant");
            let name = relative
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Filenames must be UTF-8")
                })?
                .replace('\\', "/");
            let source = fs::read_to_string(&path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
            sources.insert(name, source);
        }
    }
    Ok(())
}
/// Writes a comparison report from a rule snapshot, without changing coverage output.
pub fn generate_comparison(
    config: &Path,
    snapshot: &Path,
    output: &Path,
    inputs: &comparison::Inputs,
) -> Result<comparison::Report, Box<dyn std::error::Error>> {
    let ledger = ledger::inventory(&read_sources(config)?);
    let report =
        comparison::evaluate(&ledger, &fs::read(snapshot)?, inputs).map_err(io::Error::other)?;
    fs::create_dir_all(output)?;
    fs::write(output.join("comparison.json"), json_bytes(&report)?)?;
    Ok(report)
}
/// Writes the comparison of Native's registry list with the config types.
pub fn generate_registries(
    config: &Path,
    registries: &pdx_native::Answer<Vec<pdx_native::Registry>>,
    output: &Path,
) -> Result<coverage::registries::Report, Box<dyn std::error::Error>> {
    let ledger = ledger::inventory(&read_sources(config)?);
    let report = coverage::registries::compare(&ledger, registries);
    fs::create_dir_all(output)?;
    fs::write(output.join("registries.json"), json_bytes(&report)?)?;
    Ok(report)
}
/// Reads each `script-docs` log of one game version's directory. A missing log is an error.
pub fn read_script_docs(directory: &Path) -> io::Result<BTreeMap<String, String>> {
    comparison::script_docs::LOGS
        .iter()
        .map(|log| {
            let path = directory.join(log);
            let text = fs::read_to_string(&path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;

            Ok(((*log).to_owned(), text))
        })
        .collect()
}

/// Reads define files, keyed by their path as given. A repeated path is an error.
pub fn read_define_files(paths: &[PathBuf]) -> io::Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();

    for path in paths {
        let text = fs::read_to_string(path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;

        if files.insert(path.display().to_string(), text).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Define file given twice: {}", path.display()),
            ));
        }
    }

    Ok(files)
}

/// Reads the base-game `common/**/*.txt` files of an installation, keyed by their path relative
/// to the installation root (the directory that holds `common`). Rejects symlinks, unreadable or non-UTF-8 files, and an installation
/// with no such file.
pub fn read_game_content(installation: &Path) -> io::Result<BTreeMap<String, String>> {
    if fs::symlink_metadata(installation)?.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Installation root must not be a symlink",
        ));
    }

    let common = installation.join("common");

    if fs::symlink_metadata(&common)?.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The installation's common directory must not be a symlink",
        ));
    }

    let mut files = BTreeMap::new();

    visit(installation, &common, "txt", &mut files)?;

    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No common/**/*.txt files found in the installation",
        ));
    }

    Ok(files)
}

/// Reads all CWT files below an explicit root. Rejects symlinks and empty inputs rather than skipping them.
pub fn read_sources(root: &Path) -> io::Result<Sources> {
    if fs::symlink_metadata(root)?.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Config root must not be a symlink",
        ));
    }
    let mut sources = Sources::new();
    visit(root, root, "cwt", &mut sources)?;
    if sources.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No .cwt files found",
        ));
    }
    Ok(sources)
}
/// Serializes deterministic, pretty JSON with a final newline.
pub fn json_bytes(value: &impl serde::Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
/// Writes the ledger, the coverage report, this run's baseline (`covered/`, replaced whole) and
/// the dashboard page (`dashboard.html`). `content` is the shipped files of the snapshot's build
/// (see [`coverage::evaluate`]); `observed_live` is whether the snapshot holds a live observation.
/// The page compares the run with `baseline` and reads `comparison.json` and `registries.json`
/// from `output` when they are present.
pub fn generate(
    config: &Path,
    snapshot: Option<&Path>,
    content: Option<&coverage::Content>,
    observed_live: bool,
    output: &Path,
    baseline: Option<&Baseline>,
) -> Result<(Ledger, coverage::Report), Box<dyn std::error::Error>> {
    let sources = read_sources(config)?;
    let mut ledger = ledger::inventory(&sources);

    if let Some(content) = content {
        coverage::attribute_authored_text(&mut ledger, content);
    }

    let input = snapshot.map(fs::read).transpose()?;
    let report =
        coverage::evaluate(&ledger, input.as_deref(), content).map_err(io::Error::other)?;
    let ledger_bytes = json_bytes(&ledger)?;
    let report_bytes = json_bytes(&report)?;
    let ledger_sha256 = format!("{:x}", Sha256::digest(&ledger_bytes));
    fs::create_dir_all(output)?;
    fs::write(output.join("ledger.json"), ledger_bytes)?;
    fs::write(output.join("coverage.json"), report_bytes)?;

    let comparison = read_optional_json(&output.join("comparison.json"));
    let registries = read_optional_json(&output.join("registries.json"));
    let page = dashboard::page_data(
        &ledger,
        &ledger_sha256,
        &report,
        baseline,
        comparison.as_ref(),
        registries.as_ref(),
    );

    write_baseline(
        &output.join(BASELINE_DIRECTORY),
        &Baseline::of(&ledger, &ledger_sha256, &report, observed_live),
    )?;
    fs::write(output.join("dashboard.html"), dashboard::render_html(&page))?;

    Ok((ledger, report))
}

/// The directory of a run's baseline in its reports folder.
pub const BASELINE_DIRECTORY: &str = "covered";

/// Replaces `directory` with the text form of `baseline`, so no shard of an earlier run remains.
fn write_baseline(directory: &Path, baseline: &Baseline) -> io::Result<()> {
    match fs::remove_dir_all(directory) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    for (path, text) in baseline.render() {
        let path = directory.join(path);
        fs::create_dir_all(path.parent().expect("a shard lies inside the directory"))?;
        fs::write(path, text)?;
    }

    Ok(())
}

/// Reads the baseline in `directory`: every `.txt` file below it, keyed by its relative path.
pub fn read_baseline(directory: &Path) -> Result<Baseline, Box<dyn std::error::Error>> {
    let mut texts = BTreeMap::new();
    visit(directory, directory, "txt", &mut texts)?;

    Ok(Baseline::parse(&texts)?)
}

/// Reads a JSON report that another command may have written. An absent file is `None`; an
/// unreadable or invalid one is `None` with a warning, since the page only adds it when present.
fn read_optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(error) => {
            eprintln!("The dashboard leaves out {}: {error}", path.display());
            return None;
        }
    };

    serde_json::from_slice(&bytes)
        .inspect_err(|error| eprintln!("The dashboard leaves out {}: {error}", path.display()))
        .ok()
}
