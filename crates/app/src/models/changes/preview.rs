use drakeflake_plan::ManagedState;
use serde::Deserialize as _;
use serde_json::Value;

use crate::hub::Hub;

pub fn render(hub: &Hub, plan: &Value) -> String {
    let plan = match ManagedState::deserialize(plan) {
        Ok(plan) => plan,
        Err(err) => {
            log::warn!("preview: invalid plan from QML: {err}");
            return format!("# Invalid plan: {err}");
        }
    };

    match hub.render_plan(&plan) {
        Ok((path, rendered)) => {
            let entry = format!("### {}\n{}", path.display(), rendered.entry);
            let parts = rendered
                .parts
                .iter()
                .map(|part| format!("### {}\n{}", part.path.display(), part.text));

            std::iter::once(entry)
                .chain(parts)
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(errors) => {
            log::debug!("preview cannot render: {errors:?}");
            format!("# Cannot render:\n# {}", errors.join("\n# "))
        }
    }
}
