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

fn visit(root: &Path, dir: &Path, sources: &mut Sources) -> io::Result<()> {
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
            visit(root, &path, sources)?;
        } else if path.extension().is_some_and(|s| s == "cwt") {
            let relative = path.strip_prefix(root).expect("descendant");
            let name = relative
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Config filenames must be UTF-8",
                    )
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

/// Reads all CWT files below an explicit root. Rejects symlinks and empty inputs rather than skipping them.
pub fn read_sources(root: &Path) -> io::Result<Sources> {
    if fs::symlink_metadata(root)?.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Config root must not be a symlink",
        ));
    }
    let mut sources = Sources::new();
    visit(root, root, &mut sources)?;
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
/// Writes the ledger, the coverage report, this run's baseline (`covered-claims.txt`) and the
/// dashboard page (`dashboard.html`). The page compares the run with `baseline` and reads
/// `comparison.json` and `registries.json` from `output` when they are present.
pub fn generate(
    config: &Path,
    snapshot: Option<&Path>,
    output: &Path,
    baseline: Option<&Baseline>,
) -> Result<(Ledger, coverage::Report), Box<dyn std::error::Error>> {
    let sources = read_sources(config)?;
    let ledger = ledger::inventory(&sources);
    let input = snapshot.map(fs::read).transpose()?;
    let report = coverage::evaluate(&ledger, input.as_deref()).map_err(io::Error::other)?;
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

    fs::write(
        output.join("covered-claims.txt"),
        Baseline::of(&ledger, &ledger_sha256, &report).render(),
    )?;
    fs::write(output.join("dashboard.html"), dashboard::render_html(&page))?;

    Ok((ledger, report))
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
