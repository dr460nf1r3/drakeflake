use drakeflake_plan::Priority;
use drakeflake_validate::ValueShape;

pub const fn choices(shape: ValueShape) -> &'static [Priority] {
    match shape {
        ValueShape::Scalar => &[
            Priority::Override,
            Priority::Default,
            Priority::Normal,
            Priority::Force,
        ],
        ValueShape::List => &[
            Priority::Merge,
            Priority::Before,
            Priority::After,
            Priority::Replace,
        ],
        ValueShape::Attrs => &[Priority::Merge, Priority::Replace],
    }
}

pub fn apply(priority: Priority, value: String) -> String {
    match wrapper(priority) {
        None => value,
        // `f -1` is a subtraction in Nix.
        Some(function) if value.starts_with('-') => format!("{function} ({value})"),
        Some(function) => format!("{function} {value}"),
    }
}

const fn wrapper(priority: Priority) -> Option<&'static str> {
    match priority {
        Priority::Normal | Priority::Merge => None,
        Priority::Default => Some("lib.mkDefault"),
        Priority::Override => Some("lib.mkOverride 90"),
        Priority::Force | Priority::Replace => Some("lib.mkForce"),
        Priority::Before => Some("lib.mkBefore"),
        Priority::After => Some("lib.mkAfter"),
    }
}
