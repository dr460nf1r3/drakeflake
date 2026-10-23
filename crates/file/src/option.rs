mod priority;

use drakeflake_plan::{InputMode, ManagedOption, Priority};
use drakeflake_render::{NixValue, render_value};
use drakeflake_validate::{
    Editor, OptionType, ValueShape, parse_expression, parse_input, parse_type, value_shape,
    wrap_expression,
};

use crate::error::FileError;

pub const OVERRIDE_PRIORITY: u32 = 90;

/// Wraps an expression so the type check sees module-system properties by their content.
const UNWRAP_PROPERTIES: &str = "let __drakeflakeUnwrap = v: \
    if builtins.isAttrs v && builtins.elem (v._type or \"\") [ \"if\" \"override\" \"order\" ] \
    then __drakeflakeUnwrap v.content \
    else if builtins.isAttrs v && (v._type or \"\") == \"merge\" && v.contents != [ ] \
    then __drakeflakeUnwrap (builtins.head v.contents) \
    else v; in __drakeflakeUnwrap ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedValue {
    pub text: String,
    /// Written as the user typed it: continuation lines are never re-indented.
    pub verbatim: bool,
}

/// Kept out of drakeflake-plan, because these parts need the option's type and the renderer.
pub trait ManagedOptionExt {
    #[must_use = "only derives the shape from the type"]
    fn shape(&self) -> ValueShape;

    #[must_use = "only derives the priority from the type"]
    fn implicit_priority(&self) -> Priority;

    #[must_use = "only lists the choices"]
    fn priority_choices(&self) -> Vec<Priority>;

    #[must_use = "only returns the priority"]
    fn effective_priority(&self) -> Priority;

    /// Drop a chosen priority equal to the implicit one, so the same setting is always stored the same way.
    #[must_use = "the value is consumed and only the result holds the change"]
    fn normalized(self) -> Self;

    fn check_settings(&self, name: &str) -> Result<(), FileError>;

    fn value(&self) -> Result<RenderedValue, FileError>;

    fn render(&self, name: &str) -> Result<RenderedValue, FileError>;

    fn check_source(&self) -> Result<String, FileError>;
}

impl ManagedOptionExt for ManagedOption {
    fn shape(&self) -> ValueShape {
        value_shape(&self.type_name)
    }

    fn implicit_priority(&self) -> Priority {
        let ty = parse_type(&self.type_name);
        let value = match self.mode {
            InputMode::Typed => parse_input(&ty, &self.input).ok(),
            InputMode::Expression => None,
        };

        implicit_priority(&ty, self.shape(), self.mode, value.as_ref())
    }

    fn priority_choices(&self) -> Vec<Priority> {
        let implicit = self.implicit_priority();
        let choices = priority::choices(self.shape());
        let mut out = Vec::with_capacity(choices.len() + 1);

        if !choices.contains(&implicit) {
            out.push(implicit);
        }

        out.extend_from_slice(choices);
        out
    }

    fn effective_priority(&self) -> Priority {
        self.priority.unwrap_or_else(|| self.implicit_priority())
    }

    fn normalized(mut self) -> Self {
        if self.priority == Some(self.implicit_priority()) {
            self.priority = None;
        }

        self
    }

    fn check_settings(&self, name: &str) -> Result<(), FileError> {
        if let Some(priority) = self.priority
            && !self.priority_choices().contains(&priority)
        {
            return Err(FileError::PriorityNotApplicable(priority));
        }

        match &self.host {
            Some(host) if host.trim().is_empty() => Err(FileError::EmptyHost),
            Some(_) if !host_scope_allowed(name) => {
                Err(FileError::HostScopeNotAllowed(name.into()))
            }
            _ => Ok(()),
        }
    }

    fn value(&self) -> Result<RenderedValue, FileError> {
        match self.mode {
            InputMode::Typed => {
                let value = parse_input(&parse_type(&self.type_name), &self.input)?;

                Ok(RenderedValue {
                    text: render_value(&value),
                    verbatim: matches!(value, NixValue::Raw(_)),
                })
            }
            InputMode::Expression => Ok(RenderedValue {
                text: wrap_expression(&parse_expression(&self.input)?),
                verbatim: true,
            }),
        }
    }

    fn render(&self, name: &str) -> Result<RenderedValue, FileError> {
        self.check_settings(name)?;
        let value = self.value()?;

        Ok(RenderedValue {
            text: priority::apply(self.effective_priority(), value.text),
            verbatim: value.verbatim,
        })
    }

    fn check_source(&self) -> Result<String, FileError> {
        let value = self.value()?;

        Ok(match self.mode {
            InputMode::Typed => value.text,
            InputMode::Expression => format!("({UNWRAP_PROPERTIES}{})", value.text),
        })
    }
}

/// Never the host name itself, since the host condition reads it and would recurse.
#[must_use = "only checks the option name"]
pub fn host_scope_allowed(name: &str) -> bool {
    name != "networking.hostName" && name != "_module" && !name.starts_with("_module.")
}

fn implicit_priority(
    ty: &OptionType,
    shape: ValueShape,
    mode: InputMode,
    value: Option<&NixValue>,
) -> Priority {
    let scalar_editor = matches!(
        ty.editor,
        Editor::Bool
            | Editor::Int { min: _, max: _ }
            | Editor::Float
            | Editor::Str { multiline: false }
            | Editor::Path
            | Editor::Enum(_)
            | Editor::Package
    );

    match (shape, value) {
        (_, Some(NixValue::Null)) => Priority::Override,
        (ValueShape::List | ValueShape::Attrs, _) => Priority::Merge,
        _ if scalar_editor || mode == InputMode::Expression => Priority::Override,
        _ => Priority::Normal,
    }
}

#[cfg(test)]
mod tests;
