use super::{Method, Owner};

pub(super) fn method(ticket: &str) -> Method {
    let name = match ticket {
        "SDK-528" => "Registry discovery and loader ownership",
        "SDK-529" => "Registry item observations",
        "SDK-530" => "Root field discovery",
        "SDK-531" => "Shared-reader binding",
        "SDK-535" => "Effect and trigger declarations",
        "SDK-536" => "Modifier, scope and link declarations",
        "SDK-537" => "Localisation commands and scope links",
        "SDK-538" => "Callbacks and entry scopes",
        "SDK-539" => "Define inventory and value types",
        "SDK-540" => "Generated modifier families",
        "SDK-541" => "Field shapes and conditional constraints",
        "SDK-542" => "Nested command and control-block grammar",
        "SDK-543" => "References and dynamic names",
        "SDK-544" => "Numeric conversion and duration rules",
        "SDK-545" => "Weight and arithmetic modifier rules",
        "SDK-546" => "Localisation-key and asset naming rules",
        "SDK-547" => "Modifier application and propagation",
        "SDK-548" => "Effect and trigger argument grammars",
        "SDK-549" => "Scope context for blocks and fields",
        "SDK-550" => "Script expansion and parameter rules",
        "SDK-551" => "Custom, nested and late registry discovery",
        "SDK-552" => "Mounted file selection and duplicate definition rules",
        "SDK-554" => "Interface and graphics loaders",
        "SDK-555" => "Sound and music loaders",
        "SDK-556" => "Map and descriptor loaders",
        _ => unreachable!("unregistered roadmap method"),
    };
    Method {
        ticket: ticket.into(),
        name: name.into(),
    }
}
pub(super) fn route(file: &str, subject: &[String], property: &str, answer: &str) -> Method {
    let context = subject.join("/");
    let filename = file.rsplit('/').next().unwrap_or(file);
    let ticket = if file.starts_with("interface/")
        || file.starts_with("gfx/")
        || file.starts_with("fonts/")
    {
        "SDK-554"
    } else if file.starts_with("sound/") || filename == "music.cwt" {
        "SDK-555"
    } else if file.starts_with("map/")
        || filename.contains("descriptor")
        || filename == "dlc_list.cwt"
    {
        "SDK-556"
    } else if property == "loader_path" || filename == "overrides.cwt" {
        "SDK-552"
    } else if property == "naming_rule"
        || context.contains("/localisation/")
        || context.contains("/images/")
    {
        "SDK-546"
    } else if context.contains("/modifiers/") && context.contains("type[") {
        "SDK-540"
    } else if property == "scope_context" {
        "SDK-549"
    } else if property == "modifier_context" {
        "SDK-547"
    } else if filename == "defines.cwt" {
        "SDK-539"
    } else if filename == "on_actions.cwt" || filename == "game_rules.cwt" {
        "SDK-538"
    } else if filename == "localisation.cwt" || filename == "localisation_links.cwt" {
        "SDK-537"
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
        "SDK-536"
    } else if property == "command_existence"
        || (["declared_scopes", "documentation"].contains(&property)
            && ["effects.cwt", "triggers.cwt"].contains(&filename))
    {
        "SDK-535"
    } else if property == "declared_scopes" {
        "SDK-549"
    } else if property == "cardinality" || property == "conditional_constraint" {
        "SDK-541"
    } else if filename == "modifier_rule.cwt" || answer.contains("modifier_rule") {
        "SDK-545"
    } else if context.contains("scripted_")
        || answer.starts_with('$')
        || answer.contains("value_set[parameter")
    {
        "SDK-550"
    } else if answer.contains("single_alias_right[") || context.contains("single_alias[") {
        "SDK-542"
    } else if answer.starts_with('<')
        || answer.contains("value_set[")
        || answer.starts_with("value[")
    {
        "SDK-543"
    } else if answer.starts_with("int") || answer.starts_with("float") || answer == "value_field" {
        "SDK-544"
    } else if ["effects.cwt", "triggers.cwt"].contains(&filename) {
        "SDK-548"
    } else if property == "type_existence" {
        "SDK-528"
    } else if property == "field_existence" {
        "SDK-530"
    } else if property == "content_membership" {
        "SDK-529"
    } else if property == "nested_type" {
        "SDK-551"
    } else {
        "SDK-531"
    };
    method(ticket)
}
pub(super) fn documentation_owner(file: &str) -> (Owner, &'static str) {
    let name = file.rsplit('/').next().unwrap_or(file);
    if ["effects.cwt", "triggers.cwt"].contains(&name) {
        (
            Owner::EngineFact,
            "Expected engine declaration text; provenance pending SDK-525",
        )
    } else if file.starts_with("common/")
        || ["defines.cwt", "on_actions.cwt", "game_rules.cwt"].contains(&name)
    {
        (
            Owner::ContentDerived,
            "Expected shipped-content documentation; provenance pending SDK-525",
        )
    } else {
        (
            Owner::AuthoredText,
            "Unattributed config prose; provisional assignment pending SDK-525",
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
        "cardinality" | "cardinality_max_define" => (
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
    if context.contains("subtype[")
        && !context.contains("/localisation")
        && !context.contains("/images")
    {
        return (
            "conditional_constraint",
            Owner::EngineFact,
            "Underlying field condition is separate from CWT subtype naming",
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
