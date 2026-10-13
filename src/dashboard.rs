//! A local page that shows config coverage, its change against the baseline of `main`, and the
//! claims that lost coverage.
use crate::{
    coverage::{self, Assessment, registries::GATE_DIRECTORIES},
    ledger::{Claim, Ledger},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "dashboard/tests.rs"]
mod tests;

const TEMPLATE: &str = include_str!("dashboard.html");
const DATA_PLACEHOLDER: &str = "/*DASHBOARD_DATA*/null";

/// Areas of the config, each named with the files that make it up. Other files are registry types.
const AREAS: &[(&str, &[&str])] = &[
    ("Effects", &["effects.cwt"]),
    ("Triggers", &["triggers.cwt"]),
    ("Modifiers", &["modifiers.cwt", "modifier_categories.cwt"]),
    (
        "Scopes and links",
        &["scopes.cwt", "links.cwt", "scope_links.cwt"],
    ),
    (
        "Localisation",
        &["localisation.cwt", "localisation_links.cwt"],
    ),
    ("On_actions", &["on_actions.cwt"]),
    ("Game rules", &["game_rules.cwt"]),
];
const DEFINES_DIRECTORY: &str = "common/defines/";
const DEFINES_AREA: &str = "Defines";
const OTHER_AREA: &str = "Registry types and other files";

/// The covered claims of one coverage run, with the identity of the inputs it measured.
///
/// Its text form is the committed baseline `docs/coverage/covered-claims.txt`: four header lines,
/// then one covered claim ID per line, sorted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Baseline {
    /// Config content identity of the run.
    pub config_sha256: String,
    /// SHA-256 of the run's `ledger.json` bytes.
    pub ledger_sha256: String,
    /// Atlas-owned coverage of the run.
    pub covered: usize,
    /// Atlas-owned claims of the run.
    pub total: usize,
    /// Covered claim IDs.
    pub claims: BTreeSet<String>,
}

impl Baseline {
    /// The baseline of one run.
    pub fn of(ledger: &Ledger, ledger_sha256: &str, coverage: &coverage::Report) -> Self {
        Self {
            config_sha256: ledger.config_sha256.clone(),
            ledger_sha256: ledger_sha256.to_owned(),
            covered: coverage.totals.atlas_owned.covered,
            total: coverage.totals.atlas_owned.total,
            claims: atlas_owned_covered(ledger, coverage)
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Parses the text form. Fails on a missing or malformed header line.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let mut header = |key: &str| {
            lines
                .next()
                .and_then(|line| line.strip_prefix(key))
                .and_then(|value| value.strip_prefix(' '))
                .map(str::to_owned)
                .ok_or(format!("Baseline header `{key}` is missing"))
        };
        let config_sha256 = header("config_sha256")?;
        let ledger_sha256 = header("ledger_sha256")?;
        let count = |value: String| {
            value
                .parse::<usize>()
                .map_err(|e| format!("Baseline count `{value}`: {e}"))
        };
        let covered = count(header("atlas_owned_covered")?)?;
        let total = count(header("atlas_owned_total")?)?;
        let claims = lines.map(str::to_owned).collect();

        Ok(Self {
            config_sha256,
            ledger_sha256,
            covered,
            total,
            claims,
        })
    }

    /// The text form, with a final newline.
    pub fn render(&self) -> String {
        let mut text = format!(
            "config_sha256 {}\nledger_sha256 {}\natlas_owned_covered {}\natlas_owned_total {}\n",
            self.config_sha256, self.ledger_sha256, self.covered, self.total
        );

        for claim in &self.claims {
            text.push_str(claim);
            text.push('\n');
        }

        text
    }
}

/// The fields of a `compare` report that the page reads.
#[derive(Debug, Deserialize)]
pub struct ComparisonSummary {
    config_sha256: String,
    snapshot_sha256: String,
    entries: Vec<ComparisonEntry>,
}

#[derive(Debug, Deserialize)]
struct ComparisonEntry {
    status: String,
    claim: Option<String>,
    config_answer: Option<String>,
    #[serde(default)]
    atlas_answers: Vec<serde_json::Value>,
}

/// The fields of a `registries` report that the page reads.
#[derive(Debug, Deserialize)]
pub struct RegistryJoins {
    config_sha256: String,
    native_source: RegistrySource,
    rows: Vec<RegistryRow>,
}

#[derive(Debug, Deserialize)]
struct RegistrySource {
    build: String,
}

#[derive(Debug, Deserialize)]
struct RegistryRow {
    type_name: String,
    file: String,
    join: String,
}

/// Everything the page shows, computed from one coverage run.
#[derive(Debug, Serialize)]
pub struct PageData {
    /// Snapshot name and revision, absent for a run without a snapshot.
    pub snapshot_id: Option<String>,
    /// Atlas-owned coverage of this run.
    pub covered: usize,
    /// Atlas-owned claims of this run.
    pub total: usize,
    /// Atlas-owned coverage and claims of the baseline, absent without a baseline.
    pub baseline: Option<(usize, usize)>,
    /// Problems that limit what the numbers mean.
    pub warnings: Vec<String>,
    /// Claims that the baseline covers and this run does not.
    pub lost: Vec<LostClaim>,
    /// Claims that this run covers and the baseline does not.
    pub gained: usize,
    /// Baseline claims that this run's ledger no longer has.
    pub missing_from_ledger: usize,
    /// Each area of the config.
    pub areas: Vec<Row>,
    /// Each config type of the Milestone 5 gate population, lowest share first; among equal shares, most
    /// uncovered claims first; types with no Atlas-owned claim last.
    pub gate_types: Vec<Row>,
    /// Comparison entries by status, absent when no matching `compare` report is present.
    pub agreement: Option<BTreeMap<String, usize>>,
    /// Config claims whose Atlas answer differs from the config's, in ledger order. Empty when
    /// agreement is not measured.
    pub disagreements: Vec<Disagreement>,
}

/// A config claim whose Atlas answer differs from the config's answer.
#[derive(Debug, Serialize)]
pub struct Disagreement {
    /// Config file.
    pub file: String,
    /// Subject path, joined with `/`.
    pub subject: String,
    /// Property.
    pub property: String,
    /// The config's answer.
    pub config_answer: String,
    /// Atlas's answers as text: a string as itself, a lone `form` as its value, any other value as
    /// compact JSON without the reader identity.
    pub atlas_answers: Vec<String>,
}

/// A claim that lost coverage.
#[derive(Debug, Serialize)]
pub struct LostClaim {
    /// Claim occurrence ID.
    pub claim: String,
    /// Config file.
    pub file: String,
    /// Subject path, joined with `/`.
    pub subject: String,
    /// Property.
    pub property: String,
    /// Why this run gives no credit.
    pub reason: String,
}

/// Atlas-owned coverage of one area or config type.
#[derive(Debug, Default, Serialize)]
pub struct Row {
    /// Area or config type name.
    pub name: String,
    /// Config file of a config type.
    pub file: Option<String>,
    /// How a config type joins Native's registry list, when a matching `registries` report is
    /// present.
    pub join: Option<String>,
    /// Covered claims.
    pub covered: usize,
    /// Atlas-owned claims.
    pub total: usize,
    /// Covered claims of the baseline, absent without a baseline.
    pub baseline_covered: Option<usize>,
}

/// Computes the page from one coverage run, its baseline and the optional reports beside it.
///
/// A `compare` or `registries` report counts only when it describes the same config (and, for
/// `compare`, the same snapshot); otherwise the page warns and leaves it out.
pub fn page_data(
    ledger: &Ledger,
    ledger_sha256: &str,
    coverage: &coverage::Report,
    baseline: Option<&Baseline>,
    comparison: Option<&ComparisonSummary>,
    registries: Option<&RegistryJoins>,
) -> PageData {
    let claims: BTreeMap<&str, &Claim> = ledger.claims.iter().map(|c| (c.id.as_str(), c)).collect();
    let assessments: BTreeMap<&str, &Assessment> = coverage
        .claims
        .iter()
        .map(|assessment| (assessment.claim.as_str(), assessment))
        .collect();
    let is_covered = |id: &str| assessments.get(id).is_some_and(|a| a.covered);
    let mut warnings = Vec::new();

    if !coverage.inventory_complete {
        warnings.push(format!(
            "{} config source diagnostics remain; the numbers cover inventoried claims only.",
            ledger.diagnostics.len()
        ));
    }

    let baseline_covered = baseline.map(|baseline| {
        warnings.extend(baseline_warnings(baseline, ledger, ledger_sha256));

        baseline
            .claims
            .iter()
            .filter_map(|id| claims.get(id.as_str()).copied())
            .filter(|claim| claim.owner.atlas_owned())
            .collect::<Vec<_>>()
    });

    let lost = baseline_covered
        .iter()
        .flatten()
        .filter(|claim| !is_covered(&claim.id))
        .map(|claim| lost_claim(claim, assessments[claim.id.as_str()]))
        .collect();
    let missing_from_ledger = baseline.map_or(0, |baseline| {
        baseline
            .claims
            .iter()
            .filter(|id| !claims.contains_key(id.as_str()))
            .count()
    });
    let gained = baseline.map_or(0, |baseline| {
        atlas_owned_covered(ledger, coverage)
            .filter(|id| !baseline.claims.contains(*id))
            .count()
    });

    let joins = registries.and_then(|registries| {
        let same_target = coverage
            .target
            .as_ref()
            .is_none_or(|target| *target == registries.native_source.build);

        if registries.config_sha256 == ledger.config_sha256 && same_target {
            return Some(
                registries
                    .rows
                    .iter()
                    .map(|row| {
                        (
                            (row.type_name.as_str(), row.file.as_str()),
                            row.join.clone(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>(),
            );
        }

        warnings.push(
            "registries.json describes another config or game build; joins are left out.".into(),
        );

        None
    });

    let measured = comparison.and_then(|comparison| {
        let same_inputs = comparison.config_sha256 == coverage.config_sha256
            && Some(&comparison.snapshot_sha256) == coverage.snapshot_sha256.as_ref();

        if same_inputs {
            return Some(comparison);
        }

        warnings.push(
            "comparison.json describes another config or snapshot; agreement is not measured."
                .into(),
        );

        None
    });
    let agreement = measured.map(agreement_counts);
    let disagreements =
        measured.map_or_else(Vec::new, |comparison| disagreements(comparison, &claims));

    PageData {
        snapshot_id: coverage.snapshot_id.clone(),
        covered: coverage.totals.atlas_owned.covered,
        total: coverage.totals.atlas_owned.total,
        baseline: baseline.map(|baseline| (baseline.covered, baseline.total)),
        warnings,
        lost,
        gained,
        missing_from_ledger,
        areas: areas(coverage, baseline_covered.as_deref()),
        gate_types: gate_types(
            ledger,
            coverage,
            baseline_covered.as_deref(),
            joins.as_ref(),
        ),
        agreement,
        disagreements,
    }
}

/// Renders the page: the template with the data inside it.
pub fn render_html(data: &PageData) -> String {
    let json = serde_json::to_string(data)
        .expect("page data serializes")
        .replace('<', "\\u003c");

    TEMPLATE.replace(DATA_PLACEHOLDER, &json)
}

/// IDs of the Atlas-owned claims that `coverage` covers, the only claims that coverage counts.
fn atlas_owned_covered<'a>(
    ledger: &'a Ledger,
    coverage: &'a coverage::Report,
) -> impl Iterator<Item = &'a str> {
    let covered: BTreeSet<&str> = coverage
        .claims
        .iter()
        .filter(|assessment| assessment.covered)
        .map(|assessment| assessment.claim.as_str())
        .collect();

    ledger
        .claims
        .iter()
        .filter(move |claim| claim.owner.atlas_owned() && covered.contains(claim.id.as_str()))
        .map(|claim| claim.id.as_str())
}

fn baseline_warnings(baseline: &Baseline, ledger: &Ledger, ledger_sha256: &str) -> Vec<String> {
    if baseline.config_sha256 != ledger.config_sha256 {
        return vec![
            "The baseline is for another config; the change compares different claims.".into(),
        ];
    }

    if baseline.ledger_sha256 != ledger_sha256 {
        return vec![
            "The baseline is for another ledger; claims may have been added, removed or reclassified."
                .into(),
        ];
    }

    Vec::new()
}

fn lost_claim(claim: &Claim, assessment: &Assessment) -> LostClaim {
    LostClaim {
        claim: claim.id.clone(),
        file: claim.file.clone(),
        subject: claim.subject.join("/"),
        property: claim.property.clone(),
        reason: assessment.reason.clone(),
    }
}

fn agreement_counts(comparison: &ComparisonSummary) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();

    for entry in &comparison.entries {
        *counts.entry(entry.status.clone()).or_default() += 1;
    }

    counts
}

fn disagreements(
    comparison: &ComparisonSummary,
    claims: &BTreeMap<&str, &Claim>,
) -> Vec<Disagreement> {
    comparison
        .entries
        .iter()
        .filter(|entry| entry.status == "different")
        .filter_map(|entry| {
            let claim = claims.get(entry.claim.as_deref()?)?;

            Some(Disagreement {
                file: claim.file.clone(),
                subject: claim.subject.join("/"),
                property: claim.property.clone(),
                config_answer: entry.config_answer.clone().unwrap_or_default(),
                atlas_answers: entry.atlas_answers.iter().map(answer_text).collect(),
            })
        })
        .collect()
}

/// Keys of an Atlas answer that identify how it was read, not what it says.
const READER_KEYS: &[&str] = &["reader", "schema"];

fn answer_text(answer: &serde_json::Value) -> String {
    let serde_json::Value::Object(fields) = answer else {
        return answer
            .as_str()
            .map_or_else(|| answer.to_string(), str::to_owned);
    };
    let mut fields = fields.clone();

    fields.retain(|key, _| !READER_KEYS.contains(&key.as_str()));

    match fields.get("form") {
        Some(serde_json::Value::String(form)) if fields.len() == 1 => form.clone(),
        _ => serde_json::Value::Object(fields).to_string(),
    }
}

fn area_of(file: &str) -> &'static str {
    if file.starts_with(DEFINES_DIRECTORY) {
        return DEFINES_AREA;
    }

    AREAS
        .iter()
        .find(|(_, files)| files.contains(&file))
        .map_or(OTHER_AREA, |(name, _)| name)
}

fn areas(coverage: &coverage::Report, baseline_covered: Option<&[&Claim]>) -> Vec<Row> {
    let names = AREAS
        .iter()
        .map(|(name, _)| *name)
        .chain([DEFINES_AREA, OTHER_AREA]);
    let mut rows: Vec<Row> = names
        .map(|name| Row {
            name: name.into(),
            baseline_covered: baseline_covered.map(|_| 0),
            ..Row::default()
        })
        .collect();

    for (file, totals) in &coverage.by_file {
        let row = area_row(&mut rows, file);

        row.covered += totals.atlas_owned.covered;
        row.total += totals.atlas_owned.total;
    }

    for claim in baseline_covered.into_iter().flatten() {
        if let Some(count) = &mut area_row(&mut rows, &claim.file).baseline_covered {
            *count += 1;
        }
    }

    rows
}

fn area_row<'a>(rows: &'a mut [Row], file: &str) -> &'a mut Row {
    let area = area_of(file);

    rows.iter_mut()
        .find(|row| row.name == area)
        .expect("every area has a row")
}

fn gate_types(
    ledger: &Ledger,
    coverage: &coverage::Report,
    baseline_covered: Option<&[&Claim]>,
    joins: Option<&BTreeMap<(&str, &str), String>>,
) -> Vec<Row> {
    let file_types = coverage::file_types(ledger);
    let mut baseline_by_type: BTreeMap<(&str, &str), usize> = BTreeMap::new();

    for claim in baseline_covered.into_iter().flatten() {
        if let Some(type_name) = coverage::claim_type(claim, &file_types[&claim.file]) {
            *baseline_by_type
                .entry((type_name, claim.file.as_str()))
                .or_default() += 1;
        }
    }

    let mut rows: Vec<Row> = coverage::registry_types(ledger)
        .into_iter()
        .filter(|(directory, _)| coverage::registries::below(directory, GATE_DIRECTORIES))
        .flat_map(|(_, types)| types)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(type_name, file)| {
            let key = (type_name.as_str(), file.as_str());
            let totals = &coverage.by_type[&file].types[&type_name].atlas_owned;

            Row {
                join: joins.and_then(|joins| joins.get(&key).cloned()),
                covered: totals.covered,
                total: totals.total,
                baseline_covered: baseline_covered
                    .map(|_| baseline_by_type.get(&key).copied().unwrap_or(0)),
                name: type_name,
                file: Some(file),
            }
        })
        .collect();

    rows.sort_by(least_covered_first);

    rows
}

/// Orders rows by share ascending, comparing `covered / total` without division. A row with no
/// claim has no share and goes last; equal shares put the larger gap first.
fn least_covered_first(a: &Row, b: &Row) -> std::cmp::Ordering {
    (a.total == 0)
        .cmp(&(b.total == 0))
        .then((a.covered * b.total).cmp(&(b.covered * a.total)))
        .then((b.total - b.covered).cmp(&(a.total - a.covered)))
}
