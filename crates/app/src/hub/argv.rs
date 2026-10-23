use std::ffi::{OsStr, OsString};

pub(super) fn arg(value: impl AsRef<OsStr>) -> OsString {
    value.as_ref().to_owned()
}

pub(super) fn echo(program: &OsStr, args: &[OsString]) -> String {
    args.iter().fold(
        format!("$ {}", program.to_string_lossy()),
        |mut line, arg| {
            line.push(' ');
            line.push_str(&arg.to_string_lossy());
            line
        },
    )
}

#[cfg(test)]
mod tests;
