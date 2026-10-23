mod assemble;
mod builtins;
mod source;
#[cfg(test)]
mod tests;
mod xml;

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use drakeflake_functions::{FORMAT_VERSION, IndexFile, Source};

#[derive(Debug, Default)]
struct Args {
    positions: PathBuf,
    builtins: PathBuf,
    nixpkgs: String,
    rev: String,
    date: i64,
    nix: String,
    output: PathBuf,
}

fn main() -> ExitCode {
    let result = parse_args(std::env::args().skip(1)).and_then(|args| run(&args));
    let mut stderr = std::io::stderr();

    match result {
        // A closed stderr leaves nowhere to report to, and the exit code still tells.
        Ok(count) => {
            writeln!(stderr, "drakeflake-function-index: {count} functions").ok();
            ExitCode::SUCCESS
        }
        Err(error) => {
            writeln!(stderr, "drakeflake-function-index: {error:#}").ok();
            ExitCode::FAILURE
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args> {
    let mut parsed = Args::default();

    while let Some(flag) = args.next() {
        let value = args
            .next()
            .with_context(|| format!("{flag} needs a value"))?;

        match flag.as_str() {
            "--positions" => parsed.positions = value.into(),
            "--builtins" => parsed.builtins = value.into(),
            "--nixpkgs" => parsed.nixpkgs = value,
            "--rev" => parsed.rev = value,
            "--date" => parsed.date = value.parse().context("--date is not a number")?,
            "--nix-version" => parsed.nix = value,
            "--output" => parsed.output = value.into(),
            _ => bail!("unknown option {flag}"),
        }
    }

    if parsed.positions.as_os_str().is_empty() || parsed.output.as_os_str().is_empty() {
        bail!("--positions and --output are required");
    }

    Ok(parsed)
}

fn run(args: &Args) -> Result<usize> {
    let read = |path: &PathBuf| {
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))
    };

    let collected = xml::collected(&read(&args.positions)?)?;
    let builtins = if args.builtins.as_os_str().is_empty() {
        Vec::new()
    } else {
        builtins::parse(&read(&args.builtins)?)?
    };

    let functions = assemble::assemble(collected, builtins, args.nixpkgs.trim_end_matches('/'));
    let count = functions.len();

    let index = IndexFile {
        version: FORMAT_VERSION,
        source: Source {
            rev: args.rev.clone(),
            date: args.date,
            nix: args.nix.trim().to_owned(),
        },
        functions,
    };

    let text = serde_json::to_string(&index).context("cannot write the index as JSON")?;
    std::fs::write(&args.output, text)
        .with_context(|| format!("cannot write {}", args.output.display()))?;

    Ok(count)
}
