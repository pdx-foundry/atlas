//! The fewest occurrences of a field that Native's requirement cases answer (`FieldRequirement::
//! Cases`). Each case states a condition on the definition's other stored fields and whether a
//! definition that meets it must contain the field; a definition that meets no case has no
//! established requirement.
//!
//! The cases are read into a finite space of stored states: each field that a condition tests is
//! zero or nonzero (`FieldZero`), or holds one of the keywords that the conditions name or any
//! other keyword (`FieldKeyword`). A condition that Atlas cannot read (`Unresolved`, or a form
//! that this module does not know) leaves every answer open.

use pdx_native::{FieldCondition, RequirementCase};
use std::collections::{BTreeMap, BTreeSet};

/// One line of a config subtype's selector, `key = value`, such as `colonizable = yes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectorTest<'a> {
    /// The root field that the line tests.
    pub(crate) key: &'a str,
    /// The value that the line writes.
    pub(crate) value: &'a str,
    /// Whether the line has a `## cardinality` minimum of 0, so a definition without the key also
    /// meets it.
    pub(crate) optional: bool,
}

/// The fewest occurrences of the field in any definition: 0 where a definition can meet a case
/// that does not require the field, 1 where every case requires it and every definition meets a
/// case, and `None` otherwise or where a condition is not readable.
pub(crate) fn minimum(cases: &[RequirementCase]) -> Option<u64> {
    let cases = read_cases(cases)?;
    let space = Space::new(&cases)?;
    let states = space.states();
    let optional_somewhere = states
        .iter()
        .any(|state| cases.iter().any(|case| !case.required && case.holds(state)));

    if optional_somewhere {
        return Some(0);
    }

    let every_state_met = states
        .iter()
        .all(|state| cases.iter().any(|case| case.holds(state)));
    let every_case_required = cases.iter().all(|case| case.required);

    (every_state_met && every_case_required).then_some(1)
}

/// The fewest occurrences of the field in the definitions of a config subtype whose selector is
/// `tests`: the requirement of the one case whose condition selects exactly the same stored
/// states, as 1 or 0. `None` where no case matches the selector exactly, where matching cases
/// disagree, or where a test or condition is not readable.
pub(crate) fn subtype_minimum(
    cases: &[RequirementCase],
    tests: &[SelectorTest<'_>],
) -> Option<u64> {
    let cases = read_cases(cases)?;
    let space = Space::new(&cases)?;
    let selector = space.selector_terms(tests)?;
    let states = space.states();
    let selected = region(&states, |state| {
        selector.iter().all(|term| term.holds(state))
    });

    if selected.is_empty() {
        return None;
    }

    let answers: BTreeSet<bool> = cases
        .iter()
        .filter(|case| region(&states, |state| case.holds(state)) == selected)
        .map(|case| case.required)
        .collect();

    match answers.into_iter().collect::<Vec<_>>().as_slice() {
        [required] => Some(u64::from(*required)),
        _ => None,
    }
}

/// One term of a condition: a test of one stored field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Term<'a> {
    Zero {
        path: &'a [String],
        zero: bool,
    },
    Keyword {
        path: &'a [String],
        keyword: &'a str,
        equal: bool,
    },
}

impl<'a> Term<'a> {
    fn holds(&self, state: &State<'_>) -> bool {
        match (self, state.get(self.path())) {
            (Self::Zero { zero, .. }, Some(Value::Zero(stored))) => zero == stored,
            (Self::Keyword { keyword, equal, .. }, Some(Value::Keyword(stored))) => {
                (*stored == Some(*keyword)) == *equal
            }
            _ => false,
        }
    }

    fn path(&self) -> &'a [String] {
        match *self {
            Self::Zero { path, .. } | Self::Keyword { path, .. } => path,
        }
    }
}

/// A case read into the terms that its condition joins.
struct Case<'a> {
    terms: Vec<Term<'a>>,
    required: bool,
}

impl Case<'_> {
    fn holds(&self, state: &State<'_>) -> bool {
        self.terms.iter().all(|term| term.holds(state))
    }
}

fn read_cases(cases: &[RequirementCase]) -> Option<Vec<Case<'_>>> {
    cases
        .iter()
        .map(|case| {
            let mut terms = Vec::new();

            read_condition(&case.condition, &mut terms)?;

            Some(Case {
                terms,
                required: case.required,
            })
        })
        .collect()
}

/// Appends the terms of `condition` to `terms`; `None` for a condition that is not readable.
fn read_condition<'a>(condition: &'a FieldCondition, terms: &mut Vec<Term<'a>>) -> Option<()> {
    match condition {
        FieldCondition::Always => {}
        FieldCondition::All(conditions) => {
            for condition in conditions {
                read_condition(condition, terms)?;
            }
        }
        FieldCondition::FieldZero { path, zero } => terms.push(Term::Zero { path, zero: *zero }),
        FieldCondition::FieldKeyword {
            path,
            keyword,
            equal,
        } => terms.push(Term::Keyword {
            path,
            keyword,
            equal: *equal,
        }),
        _ => return None,
    }

    Some(())
}

/// The stored value of one tested field: zero or not, or a named keyword (`None` for any keyword
/// that no condition names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value<'a> {
    Zero(bool),
    Keyword(Option<&'a str>),
}

type State<'a> = BTreeMap<&'a [String], Value<'a>>;

/// What the conditions test about one field.
enum Dimension<'a> {
    Zero,
    Keyword(BTreeSet<&'a str>),
}

/// The fields that the cases test, each with the values that tell the conditions apart.
struct Space<'a> {
    dimensions: BTreeMap<&'a [String], Dimension<'a>>,
}

impl<'a> Space<'a> {
    /// `None` where one field is tested both as zero and as a keyword.
    fn new(cases: &[Case<'a>]) -> Option<Self> {
        let mut dimensions = BTreeMap::new();

        for term in cases.iter().flat_map(|case| &case.terms) {
            let dimension = dimensions.entry(term.path()).or_insert_with(|| match term {
                Term::Zero { .. } => Dimension::Zero,
                Term::Keyword { .. } => Dimension::Keyword(BTreeSet::new()),
            });

            match (term, dimension) {
                (Term::Zero { .. }, Dimension::Zero) => {}
                (Term::Keyword { keyword, .. }, Dimension::Keyword(keywords)) => {
                    keywords.insert(*keyword);
                }
                _ => return None,
            }
        }

        Some(Self { dimensions })
    }

    /// Every combination of the values of the tested fields.
    fn states(&self) -> Vec<State<'a>> {
        let mut states = vec![State::new()];

        for (path, dimension) in &self.dimensions {
            let values: Vec<_> = match dimension {
                Dimension::Zero => vec![Value::Zero(true), Value::Zero(false)],
                Dimension::Keyword(keywords) => keywords
                    .iter()
                    .map(|keyword| Value::Keyword(Some(keyword)))
                    .chain([Value::Keyword(None)])
                    .collect(),
            };

            states = states
                .into_iter()
                .flat_map(|state| {
                    values.iter().map(move |value| {
                        let mut state = state.clone();

                        state.insert(path, *value);
                        state
                    })
                })
                .collect();
        }

        states
    }

    /// The terms of a subtype selector. `None` unless each line tests a field that a condition
    /// tests in the same way: `yes` or `no` for a zero test, where an optional `no` also holds for
    /// a definition without the key (its stored default is zero), or a keyword that a condition
    /// names.
    fn selector_terms(&self, tests: &[SelectorTest<'_>]) -> Option<Vec<Term<'a>>> {
        if tests.is_empty() {
            return None;
        }

        tests
            .iter()
            .map(|test| {
                let (path, dimension) = self
                    .dimensions
                    .iter()
                    .find(|(path, _)| matches!(path, [key] if key == test.key))?;

                match (dimension, test.value) {
                    (Dimension::Zero, "no") => Some(Term::Zero { path, zero: true }),
                    (Dimension::Zero, "yes") if !test.optional => {
                        Some(Term::Zero { path, zero: false })
                    }
                    (Dimension::Keyword(keywords), value) if !test.optional => {
                        let keyword = keywords.get(value)?;

                        Some(Term::Keyword {
                            path,
                            keyword,
                            equal: true,
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

/// The indexes of the `states` where `holds` is true.
fn region(states: &[State<'_>], holds: impl Fn(&State<'_>) -> bool) -> BTreeSet<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, state)| holds(state))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero(path: &str, zero: bool) -> FieldCondition {
        FieldCondition::FieldZero {
            path: vec![path.into()],
            zero,
        }
    }

    fn keyword(path: &str, keyword: &str, equal: bool) -> FieldCondition {
        FieldCondition::FieldKeyword {
            path: vec![path.into()],
            keyword: keyword.into(),
            equal,
        }
    }

    fn case(condition: FieldCondition, required: bool) -> RequirementCase {
        RequirementCase {
            condition,
            required,
        }
    }

    fn test<'a>(key: &'a str, value: &'a str) -> SelectorTest<'a> {
        SelectorTest {
            key,
            value,
            optional: false,
        }
    }

    #[test]
    fn a_satisfiable_optional_case_answers_zero() {
        let cases = [
            case(zero("stages", true), false),
            case(zero("stages", false), true),
        ];

        assert_eq!(minimum(&cases), Some(0));
    }

    #[test]
    fn required_cases_that_cover_every_definition_answer_one() {
        let other = FieldCondition::All(vec![
            keyword("target", "country", false),
            keyword("target", "planet", false),
        ]);
        let cases = [
            case(keyword("target", "country", true), true),
            case(keyword("target", "planet", true), true),
            case(other, true),
        ];

        assert_eq!(minimum(&cases), Some(1));
    }

    #[test]
    fn required_cases_that_leave_a_definition_unmatched_stay_open() {
        let cases = [
            case(keyword("category", "minor_patron", true), true),
            case(keyword("category", "major_patron", true), true),
            case(keyword("category", "great_patron", true), true),
        ];

        assert_eq!(minimum(&cases), None);
    }

    #[test]
    fn an_unsatisfiable_optional_case_does_not_answer_zero() {
        let contradiction = FieldCondition::All(vec![zero("star", true), zero("star", false)]);
        let cases = [case(contradiction, false), case(zero("star", true), true)];

        assert_eq!(minimum(&cases), None);
    }

    #[test]
    fn an_unreadable_condition_leaves_every_answer_open() {
        let cases = [
            case(FieldCondition::Unresolved, false),
            case(zero("star", true), false),
        ];

        assert_eq!(minimum(&cases), None);
        assert_eq!(subtype_minimum(&cases, &[test("star", "no")]), None);
    }

    #[test]
    fn a_field_tested_as_zero_and_as_a_keyword_leaves_every_answer_open() {
        let cases = [
            case(zero("class", true), false),
            case(keyword("class", "big", true), true),
        ];

        assert_eq!(minimum(&cases), None);
    }

    #[test]
    fn a_selector_equal_to_a_case_takes_its_requirement() {
        let cases = [
            case(zero("star", false), true),
            case(zero("star", true), false),
        ];

        assert_eq!(subtype_minimum(&cases, &[test("star", "yes")]), Some(1));
        assert_eq!(subtype_minimum(&cases, &[test("star", "no")]), Some(0));
    }

    #[test]
    fn an_optional_no_selects_the_zero_state() {
        let cases = [
            case(zero("star", true), true),
            case(zero("star", false), false),
        ];
        let absent_or_no = SelectorTest {
            optional: true,
            ..test("star", "no")
        };
        let absent_or_yes = SelectorTest {
            optional: true,
            ..test("star", "yes")
        };

        assert_eq!(subtype_minimum(&cases, &[absent_or_no]), Some(1));
        assert_eq!(subtype_minimum(&cases, &[absent_or_yes]), None);
    }

    #[test]
    fn a_selector_wider_than_every_case_stays_open() {
        let probed = |colonizable: bool, star: bool| {
            let condition =
                FieldCondition::All(vec![zero("colonizable", !colonizable), zero("star", !star)]);

            case(condition, colonizable)
        };
        // No case states a colonizable star.
        let cases = [
            probed(true, false),
            probed(false, true),
            probed(false, false),
        ];

        assert_eq!(subtype_minimum(&cases, &[test("colonizable", "yes")]), None);
        assert_eq!(
            subtype_minimum(&cases, &[test("colonizable", "yes"), test("star", "no")]),
            Some(1)
        );
    }

    #[test]
    fn a_selector_keyword_matches_only_a_named_keyword() {
        let cases = [
            case(keyword("target", "country", true), true),
            case(keyword("target", "country", false), false),
        ];

        assert_eq!(
            subtype_minimum(&cases, &[test("target", "country")]),
            Some(1)
        );
        assert_eq!(subtype_minimum(&cases, &[test("target", "scalar")]), None);
    }

    #[test]
    fn a_selector_on_a_field_that_no_case_tests_stays_open() {
        let cases = [
            case(zero("star", false), true),
            case(zero("star", true), false),
        ];

        assert_eq!(subtype_minimum(&cases, &[test("ringworld", "yes")]), None);
        assert_eq!(subtype_minimum(&cases, &[]), None);
    }
}
