//! Qt's Markdown renders code blocks plainly, so they are split out for the app's highlighted code view.

use serde::Serialize;

const LABELLED: [(&str, &str); 5] = [
    (".note", "Note:"),
    (".warning", "Warning:"),
    (".tip", "Tip:"),
    (".important", "Important:"),
    (".caution", "Caution:"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Block {
    Heading { level: u8, text: String },
    Text { text: String },
    Code { language: String, text: String },
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    text: Vec<String>,
}

impl Builder {
    fn flush(&mut self) {
        let text = self.text.join("\n").trim().to_owned();
        self.text.clear();

        if !text.is_empty() {
            self.blocks.push(Block::Text { text });
        }
    }

    fn push(&mut self, block: Block) {
        self.flush();
        self.blocks.push(block);
    }
}

#[must_use = "parsing the documentation has no other effect"]
pub fn blocks(doc: &str, signature: &str) -> Vec<Block> {
    let mut builder = Builder::default();
    let mut lines = doc.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if let Some(fence) = fence_of(trimmed) {
            let language = trimmed.trim_start_matches(fence).trim().to_owned();
            let code: Vec<&str> = lines
                .by_ref()
                .take_while(|code| !code.trim_start().starts_with(fence))
                .collect();

            builder.push(Block::Code {
                language,
                text: dedent(&code),
            });
        } else if let Some(admonition) = trimmed.strip_prefix(":::") {
            let class = admonition.trim().trim_matches(|ch| ch == '{' || ch == '}');
            if let Some((_, label)) = LABELLED.iter().find(|(name, _)| class == *name) {
                builder.flush();
                builder.text.push(format!("**{label}**"));
            }
        } else if let Some((level, text)) = heading(trimmed) {
            builder.push(Block::Heading {
                level,
                text: text.to_owned(),
            });
        } else {
            builder.text.push(link_targets(line));
        }
    }

    builder.flush();

    if signature.is_empty() {
        builder.blocks
    } else {
        without_type_section(builder.blocks)
    }
}

fn fence_of(line: &str) -> Option<&'static str> {
    ["```", "~~~"]
        .into_iter()
        .find(|fence| line.starts_with(fence))
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    let text = line.get(hashes..)?;

    if (1..=6).contains(&hashes) && text.starts_with(' ') {
        Some((u8::try_from(hashes).ok()?, text.trim()))
    } else {
        None
    }
}

fn without_type_section(blocks: Vec<Block>) -> Vec<Block> {
    let mut kept = Vec::with_capacity(blocks.len());
    let mut skipping = false;

    for block in blocks {
        match &block {
            Block::Heading { text, level: _ } if text == "Type" => {
                skipping = true;
                continue;
            }
            Block::Code {
                language: _,
                text: _,
            } if skipping => {
                skipping = false;
                continue;
            }
            Block::Heading { level: _, text: _ } => skipping = false,
            Block::Text { text: _ }
            | Block::Code {
                language: _,
                text: _,
            } => {}
        }

        kept.push(block);
    }

    kept
}

fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| line.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned()
}

/// nixpkgs links functions as `[](#function-library-lib.strings.concatStrings)`.
/// Qt shows an empty label as nothing: the function's path becomes the label.
fn link_targets(line: &str) -> String {
    const EMPTY: &str = "[](#function-library-";

    let mut result = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(start) = rest.find(EMPTY) {
        let (before, found) = rest.split_at(start);
        let target = found.get(EMPTY.len()..).unwrap_or("");
        let end = target.find(')').unwrap_or(target.len());
        let path = target.get(..end).unwrap_or("");

        result.push_str(before);
        result.push_str("[`");
        result.push_str(path);
        result.push_str("`](#function-library-");
        result.push_str(path);
        result.push(')');
        rest = target.get(end + 1..).unwrap_or("");
    }

    result.push_str(rest);
    result
}
