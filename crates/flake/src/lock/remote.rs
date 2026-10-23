use std::time::Duration;

use drakeflake_exec::{Captured, Context, ExecError, Invocation, redact_urls};

use crate::error::LockError;

const REMOTE_TIMEOUT: Duration = Duration::from_secs(30);

pub fn remote_rev(
    exec: &Context,
    remote: &str,
    reference: Option<&str>,
) -> Result<String, LockError> {
    let pattern = reference.unwrap_or("HEAD");

    // No password prompts: a private repository just fails the check.
    let args = [
        "GIT_TERMINAL_PROMPT=0",
        "GIT_ASKPASS=",
        "SSH_ASKPASS=",
        "GIT_SSH_COMMAND=ssh -oBatchMode=yes",
        "git",
        "ls-remote",
        "--",
        remote,
        pattern,
    ];

    let captured = exec.run(Invocation::new("env", args, REMOTE_TIMEOUT))?;

    ls_remote_rev(&captured, reference)
}

fn ls_remote_rev(captured: &Captured, reference: Option<&str>) -> Result<String, LockError> {
    let pattern = reference.unwrap_or("HEAD");

    if !captured.success {
        return Err(LockError::Git(
            captured
                .stderr
                .lines()
                .last()
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map_or_else(|| "git ls-remote failed".to_owned(), str::to_owned),
        ));
    }

    pick_rev(&captured.stdout, reference)
        .ok_or_else(|| LockError::NotFoundUpstream(pattern.to_owned()))
}

pub fn pick_rev(output: &str, reference: Option<&str>) -> Option<String> {
    let refs: Vec<(&str, &str)> = output
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();

    let wanted = reference.map_or_else(
        || vec!["HEAD".to_owned()],
        |name| {
            vec![
                format!("refs/heads/{name}"),
                format!("refs/tags/{name}^{{}}"),
                format!("refs/tags/{name}"),
                name.to_owned(),
            ]
        },
    );

    wanted
        .iter()
        .find_map(|name| refs.iter().find(|(_, listed)| listed == name))
        .map(|(rev, _)| (*rev).to_owned())
}

pub fn immutable_link(exec: &Context, url: &str) -> Result<Option<String>, LockError> {
    let max_time = REMOTE_TIMEOUT.as_secs().to_string();
    let args = [
        "-sSIL",
        "--max-time",
        &max_time,
        "--proto",
        "=https,http",
        "--proto-redir",
        "=https,http",
        "--",
        url,
    ];

    let captured = exec
        .run(Invocation::new("curl", args, REMOTE_TIMEOUT))
        .map_err(|error| match error {
            ExecError::Spawn {
                program: _,
                kind: _,
            } => LockError::CurlUnavailable(error),
            ExecError::Cancelled(_)
            | ExecError::TimedOut {
                program: _,
                after: _,
            }
            | ExecError::Wait {
                program: _,
                kind: _,
            }
            | ExecError::Failed(_) => LockError::Exec(error),
        })?;

    if !captured.success {
        return Err(LockError::Curl(captured.stderr.trim().to_owned()));
    }

    let link = parse_immutable_link(&captured.stdout);

    log::debug!(
        "immutable link of {}: {}",
        redact_urls(url),
        redact_urls(&format!("{link:?}"))
    );
    Ok(link)
}

pub fn parse_immutable_link(headers: &str) -> Option<String> {
    headers
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim().eq_ignore_ascii_case("link").then_some(value)
        })
        .flat_map(|value| split_link_entries(value))
        .filter(|entry| {
            let lower = entry.to_ascii_lowercase();
            lower.contains("rel=\"immutable\"") || lower.contains("rel='immutable'")
        })
        .filter_map(|entry| {
            let start = entry.find('<')?;
            let rest = entry.get(start..)?;
            let end = rest.find('>')?;
            entry.get(start + 1..start + end).map(str::to_owned)
        })
        .next_back()
}

fn split_link_entries(value: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut depth = 0_usize;
    let mut quote = None;
    let mut start = 0_usize;

    for (index, byte) in value.bytes().enumerate() {
        match byte {
            b'<' if quote.is_none() => depth += 1,
            b'>' if quote.is_none() => depth = depth.saturating_sub(1),
            b'"' | b'\'' if depth == 0 => {
                if quote == Some(byte) {
                    quote = None;
                } else {
                    quote = quote.or(Some(byte));
                }
            }
            b',' if depth == 0 && quote.is_none() => {
                if let Some(part) = value.get(start..index) {
                    entries.push(part.trim());
                }

                start = index + 1;
            }
            _ => {}
        }
    }

    if let Some(part) = value.get(start..) {
        entries.push(part.trim());
    }

    entries
        .into_iter()
        .filter(|entry| !entry.is_empty())
        .collect()
}

pub fn without_query(url: &str) -> &str {
    url.split_once('?').map_or(url, |(path, _)| path)
}

fn percent_decode(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    out.push(byte);
                } else {
                    out.push(b'%');
                    out.extend_from_slice(hex.as_bytes());
                }
            }
            '+' => out.push(b' '),
            _ => {
                let mut buffer = [0_u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

pub fn query_param(url: &str, key: &str) -> Option<String> {
    url.split_once('?')?
        .1
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| percent_decode(value))
}

pub fn release_name(url: &str) -> String {
    let path = without_query(url);
    path.rsplit('/').nth(1).unwrap_or(path).to_owned()
}

#[cfg(test)]
mod tests;
