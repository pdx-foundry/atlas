use super::{Method, Owner};

pub(super) fn route(file: &str, subject: &[String], property: &str, answer: &str) -> Method {
    let context = subject.join("/");
    let filename = file.rsplit('/').next().unwrap_or(file);
    let name = if file.starts_with("interface/")
        || file.starts_with("gfx/")
        || file.starts_with("fonts/")
    {
        "Interface and graphics loaders"
    } else if file.starts_with("sound/") || filename == "music.cwt" {
        "Sound and music loaders"
    } else if file.starts_with("map/")
        || filename.contains("descriptor")
        || filename == "dlc_list.cwt"
    {
        "Map and descriptor loaders"
    } else if property == "loader_path" || filename == "overrides.cwt" {
        "Mounted file selection and duplicate definition rules"
    } else if property == "naming_rule"
        || context.contains("/localisation/")
        || context.contains("/images/")
    {
        "Localisation-key and asset naming rules"
    } else if context.contains("/modifiers/") && context.contains("type[") {
        "Generated modifier families"
    } else if property == "scope_context" {
        "Scope context for blocks and fields"
    } else if property == "modifier_context" {
        "Modifier application and propagation"
    } else if filename == "defines.cwt" {
        "Define inventory and value types"
    } else if filename == "on_actions.cwt" || filename == "game_rules.cwt" {
        "Callbacks and entry scopes"
    } else if filename == "localisation.cwt" || filename == "localisation_links.cwt" {
        "Localisation commands and scope links"
    } else if [
        "modifiers.cwt",
        "modifier_categories.cwt",
        "scopes.cwt",
        "links.cwt",
        "scope_links.cwt",
    ]
    .contains(&filename)
        && !file.starts_with("common/")
    {
        "Modifier, scope and link declarations"
    } else if property == "command_existence"
        || (["declared_scopes", "documentation"].contains(&property)
            && ["effects.cwt", "triggers.cwt"].contains(&filename))
    {
        "Effect and trigger declarations"
    } else if property == "declared_scopes" {
        "Scope context for blocks and fields"
    } else if property.starts_with("cardinality") || property == "conditional_constraint" {
        "Field shapes and conditional constraints"
    } else if filename == "modifier_rule.cwt" || answer.contains("modifier_rule") {
        "Weight and arithmetic modifier rules"
    } else if context.contains("scripted_")
        || answer.starts_with('$')
        || answer.contains("value_set[parameter")
    {
        "Script expansion and parameter rules"
    } else if answer.contains("single_alias_right[") || context.contains("single_alias[") {
        "Nested command and control-block grammar"
    } else if answer.starts_with('<')
        || answer.contains("value_set[")
        || answer.starts_with("value[")
    {
        "References and dynamic names"
    } else if answer.starts_with("int") || answer.starts_with("float") || answer == "value_field" {
        "Numeric conversion and duration rules"
    } else if ["effects.cwt", "triggers.cwt"].contains(&filename) {
        "Effect and trigger argument grammars"
    } else if property == "type_existence" {
        "Registry discovery and loader ownership"
    } else if property == "field_existence" {
        "Root field discovery"
    } else if property == "content_membership" {
        "Registry item observations"
    } else if property == "nested_type" {
        "Custom, nested and late registry discovery"
    } else {
        "Shared-reader binding"
    };
    Method { name: name.into() }
}
pub(super) fn documentation_owner(file: &str) -> (Owner, &'static str) {
    let name = file.rsplit('/').next().unwrap_or(file);
    if ["effects.cwt", "triggers.cwt"].contains(&name) {
        (
            Owner::EngineFact,
            "Expected engine declaration text; attribution is outside the current ledger",
        )
    } else if file.starts_with("common/")
        || ["defines.cwt", "on_actions.cwt", "game_rules.cwt"].contains(&name)
    {
        (
            Owner::ContentDerived,
            "Expected shipped-content documentation; attribution is outside the current ledger",
        )
    } else {
        (
            Owner::AuthoredText,
            "Unattributed config prose; provisional assignment",
        )
    }
}
pub(super) fn annotation_property(
    name: &str,
    value: &str,
) -> Option<(&'static str, Owner, &'static str)> {
    Some(match name {
        "cardinality" if value.contains('~') => (
            "soft_cardinality",
            Owner::ConsumerPolicy,
            "Soft occurrence recommendation belongs to the consumer",
        ),
        "cardinality_max_define" => (
            "cardinality_maximum_reference",
            Owner::EngineFact,
            "Define-derived occurrence bound requires engine evidence",
        ),
        "cardinality" => (
            "cardinality",
            Owner::EngineFact,
            "Occurrence constraint requires engine evidence",
        ),
        "scopes" => (
            "declared_scopes",
            Owner::EngineFact,
            "Declared applicable scopes require evidence",
        ),
        "scope" | "push_scope" | "replace_scope" | "replace_scopes" => (
            "scope_context",
            Owner::EngineFact,
            "Scope entry/change is a game question",
        ),
        "modifier_categories" => (
            "modifier_context",
            Owner::EngineFact,
            "Modifier applicability requires evidence",
        ),
        "event_type" => (
            "callback_event_type",
            Owner::EngineFact,
            "Callback kind is an engine contract",
        ),
        "optional" | "required" => (
            "naming_requirement",
            Owner::EngineFact,
            "Naming requirement requires engine evidence",
        ),
        "file_extensions" => (
            "loader_path",
            Owner::EngineFact,
            "Loader file selection requires evidence",
        ),
        "severity"
        | "group"
        | "primary"
        | "display_name"
        | "api_status"
        | "graph_related_types"
        | "per_definition"
        | "tag"
        | "incomingReferenceLabel"
        | "abbreviation" => (
            "consumer_annotation",
            Owner::ConsumerPolicy,
            "Authoring-tool presentation/modeling metadata",
        ),
        "type_key_filter" => (
            "type_filter",
            Owner::ConsumerPolicy,
            "CWT type partition; not itself an engine constraint",
        ),
        "color_type" => (
            "value_form",
            Owner::EngineFact,
            "Color representation is a reader question",
        ),
        _ => return None,
    })
}

pub(super) fn node(
    file: &str,
    path: &[String],
    key: Option<&str>,
    answer: &str,
) -> (&'static str, Owner, &'static str) {
    let key = key.unwrap_or("");
    let context = path.join("/");
    if [
        "types",
        "enums",
        "values",
        "links",
        "scopes",
        "scope_groups",
        "modifiers",
        "modifier_categories",
        "localisation_links",
        "localisation",
        "images",
    ]
    .contains(&key)
        && answer == "block"
    {
        return (
            "model_group",
            Owner::ConsumerPolicy,
            "CWT organization groups source questions",
        );
    }
    if context.starts_with("scope_groups")
        || (context.starts_with("scopes/") && (key == "aliases" || context.contains("/aliases")))
    {
        return (
            "scope_alias",
            Owner::ConsumerPolicy,
            "Scope aliases and groups are CWT naming/factoring",
        );
    }
    if path.len() == 1
        && [
            "modifiers",
            "scopes",
            "links",
            "localisation_links",
            "modifier_categories",
        ]
        .contains(&path[0].as_str())
    {
        return (
            "declaration_existence",
            Owner::EngineFact,
            "Language inventory declaration requires engine evidence",
        );
    }
    if file == "modifiers.cwt" && key.is_empty() {
        return (
            "modifier_category",
            Owner::EngineFact,
            "Declared category tag; does not establish modifier application",
        );
    }
    if [
        "input_scopes",
        "output_scope",
        "supported_scopes",
        "is_subscope_of",
    ]
    .contains(&key)
        || path
            .iter()
            .any(|p| ["input_scopes", "supported_scopes"].contains(&p.as_str()))
    {
        return (
            "declared_scopes",
            Owner::EngineFact,
            "Declared scope relation requires evidence",
        );
    }
    if key.starts_with("type[") {
        return (
            "type_existence",
            Owner::EngineFact,
            "A declared definition family requires engine discovery",
        );
    }
    if key.starts_with("subtype[") {
        return (
            "subtype_partition",
            Owner::ConsumerPolicy,
            "Subtype naming and partition are CWT modeling",
        );
    }
    if key.starts_with("alias[effect:") || key.starts_with("alias[trigger:") {
        if key.contains('<') {
            return (
                "alias_factoring",
                Owner::ConsumerPolicy,
                "Alias over content references is a CWT composition choice",
            );
        }
        return (
            "command_existence",
            Owner::EngineFact,
            "Command declaration is distinct from its argument grammar",
        );
    }
    if key.starts_with("alias[")
        || key.starts_with("single_alias[")
        || key.starts_with("alias_name[")
        || answer.starts_with("alias_match_left[")
    {
        return (
            "alias_factoring",
            Owner::ConsumerPolicy,
            "Alias factoring is CWT modeling, not an engine rule",
        );
    }
    if key.starts_with("complex_enum[") {
        return (
            "content_query",
            Owner::ContentDerived,
            "Enum populated from content is not a closed engine value set",
        );
    }
    if context.contains("complex_enum[") {
        return (
            "content_query",
            Owner::ContentDerived,
            "Content enumeration recipe, not an exhaustive engine rule",
        );
    }
    if key.starts_with("enum[") {
        return (
            "enum_definition",
            Owner::ConsumerPolicy,
            "Named enum grouping is CWT modeling; member questions are separate",
        );
    }
    if context.contains("enum[") {
        return (
            "enum_member",
            Owner::EngineFact,
            "Whether this enum member is accepted requires engine evidence",
        );
    }
    if context.contains("/localisation") || context.contains("/images") {
        return (
            "naming_rule",
            Owner::EngineFact,
            "Derived localisation or asset name requires evidence",
        );
    }
    if context.starts_with("types/") && context.contains("subtype[") {
        return (
            "conditional_constraint",
            Owner::EngineFact,
            "Type metadata selects a subtype using a game-field predicate",
        );
    }
    if context.contains("type[") {
        if [
            "path",
            "path_extension",
            "path_strict",
            "skip_root_key",
            "name_field",
            "type_per_file",
            "path_file",
            "name_from_file",
        ]
        .contains(&key)
        {
            return (
                "loader_path",
                Owner::EngineFact,
                "Definition discovery and naming rule requires loader evidence",
            );
        }
        if ["base_type", "unique", "severity", "graph_related_types"].contains(&key) {
            return (
                "type_model",
                Owner::ConsumerPolicy,
                "CWT type-model relationship",
            );
        }
    }
    if file == "dlc_list.cwt" || context.starts_with("values") {
        return (
            "content_membership",
            Owner::ContentDerived,
            "Inventory member from examined content, not exhaustive game acceptance",
        );
    }
    if key.is_empty() {
        return (
            "value_form",
            Owner::EngineFact,
            "Bare value form requires reader evidence",
        );
    }
    (
        "field_existence",
        Owner::EngineFact,
        "Whether the engine reads this field is a distinct question",
    )
}

pub(super) fn annotation_problem(name: &str, remainder: &str, value: &str) -> Option<String> {
    // Unknown annotations already receive their own diagnostic and remain uninterpreted.
    annotation_property(name, value)?;
    if remainder.is_empty() {
        return (!matches!(
            name,
            "optional" | "required" | "primary" | "per_definition" | "tag"
        ))
        .then(|| format!("Annotation {name} requires a value"));
    }
    if !remainder.starts_with('=') && !remainder.starts_with("<>") {
        return Some(format!(
            "Annotation {name} requires '=' or '<>' before its value"
        ));
    }
    if value.is_empty() {
        return Some(format!("Annotation {name} has an empty value"));
    }
    if name == "cardinality" {
        let range = value.split('#').next().unwrap_or(value).trim();
        let valid_bound = |bound: &str| {
            let bound = bound.strip_prefix('~').unwrap_or(bound);
            bound == "inf" || (!bound.is_empty() && bound.bytes().all(|b| b.is_ascii_digit()))
        };
        if !range
            .split_once("..")
            .is_some_and(|(min, max)| valid_bound(min) && valid_bound(max))
        {
            return Some(format!("Uninterpreted cardinality range: {value}"));
        }
    }
    if value.starts_with('{') {
        let parsed = pdxscript::cwt::parse(&format!("annotation = {value}"), "<annotation>");
        if !parsed.diagnostics.is_empty() {
            return Some(format!("Malformed annotation block: {value}"));
        }
    }
    None
}
