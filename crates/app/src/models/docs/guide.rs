//! Splits the guide into blocks because Qt's Markdown cannot do notes, scaled images or the app's code view.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::Lines;

use serde::Serialize;

const FENCES: [&str; 2] = ["```", "~~~"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct Entry {
    pub part: String,
    pub title: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(super) enum Block {
    Heading {
        level: u8,
        text: String,
        anchor: String,
    },
    Text {
        text: String,
    },
    Code {
        language: String,
        text: String,
    },
    Image {
        source: String,
        alt: String,
        width: u32,
        natural_width: u32,
        natural_height: u32,
    },
    Note {
        tone: Tone,
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Tone {
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct Page {
    pub file: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Default)]
struct Builder {
    title: String,
    blocks: Vec<Block>,
    text: Vec<String>,
    quote: Vec<String>,
    anchors: HashMap<String, usize>,
}

impl Builder {
    fn flush(&mut self) {
        let text = self.text.join("\n").trim().to_owned();
        self.text.clear();

        if !text.is_empty() {
            self.blocks.push(Block::Text { text });
        }

        let quote = self.quote.join("\n").trim().to_owned();
        self.quote.clear();

        if !quote.is_empty() {
            let tone = if quote.starts_with("**Warning:**") {
                Tone::Warning
            } else {
                Tone::Info
            };

            self.blocks.push(Block::Note { tone, text: quote });
        }
    }

    fn push(&mut self, block: Block) {
        self.flush();
        self.blocks.push(block);
    }

    fn anchor(&mut self, text: &str) -> String {
        let base = anchor_of(text);
        let seen = self.anchors.entry(base.clone()).or_insert(0);
        let anchor = if *seen == 0 {
            base
        } else {
            format!("{base}-{seen}")
        };

        *seen += 1;
        anchor
    }

    fn code(&mut self, opening: &str, fence: &str, lines: &mut Lines<'_>) {
        let language = opening.trim_start_matches(fence).trim().to_owned();
        let code: Vec<&str> = lines
            .take_while(|code| !code.trim_start().starts_with(fence))
            .collect();

        self.push(Block::Code {
            language,
            text: dedent(&code),
        });
    }

    fn quoted(&mut self, quoted: &str) {
        if !self.text.is_empty() {
            self.flush();
        }

        self.quote.push(quoted.trim_start().to_owned());
    }

    fn heading(&mut self, level: u8, text: &str) {
        if level == 1 && self.title.is_empty() {
            self.flush();
            text.clone_into(&mut self.title);
        } else {
            let anchor = self.anchor(text);
            self.push(Block::Heading {
                level,
                text: text.to_owned(),
                anchor,
            });
        }
    }

    fn prose(&mut self, line: &str) {
        if !self.quote.is_empty() {
            self.flush();
        }

        self.text.push(without_paragraph_tags(line));
    }
}

pub(super) fn dir() -> PathBuf {
    let path = std::env::var_os("DRAKEFLAKE_DOCS_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/src"),
        PathBuf::from,
    );

    path.canonicalize().unwrap_or(path)
}

#[must_use = "parsing the summary has no other effect"]
pub(super) fn contents(summary: &str) -> Vec<Entry> {
    let mut part = String::new();
    let mut entries = Vec::new();

    for line in summary.lines() {
        let trimmed = line.trim();

        if let Some(title) = trimmed.strip_prefix("# ") {
            if title != "Summary" {
                title.trim().clone_into(&mut part);
            }
        } else if let Some((title, file)) = link(trimmed.trim_start_matches("- ")) {
            entries.push(Entry {
                part: part.clone(),
                title: title.to_owned(),
                file: file.to_owned(),
            });
        } else {
            log::trace!("not a summary entry: {trimmed:?}");
        }
    }

    entries
}

/// Refuses paths and other extensions before they reach the file system.
pub(super) fn is_page_name(file: &str) -> bool {
    file.strip_suffix(".md").is_some_and(|stem| {
        !stem.is_empty()
            && stem
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    })
}

#[must_use = "parsing the page has no other effect"]
pub(super) fn page(file: &str, markdown: &str, base: &Path) -> Page {
    let mut builder = Builder::default();
    let mut lines = markdown.lines();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if let Some(fence) = FENCES.into_iter().find(|fence| trimmed.starts_with(fence)) {
            builder.code(trimmed, fence, &mut lines);
        } else if let Some(quoted) = trimmed.strip_prefix('>') {
            builder.quoted(quoted);
        } else if let Some((level, text)) = heading(trimmed) {
            builder.heading(level, text);
        } else if let Some(image) = image(trimmed, base) {
            builder.push(image);
        } else if is_paragraph_tag(trimmed) {
            builder.flush();
        } else if !builder.quote.is_empty() && !trimmed.is_empty() {
            builder.quote.push(trimmed.to_owned());
        } else {
            builder.prose(line);
        }
    }

    builder.flush();

    Page {
        file: file.to_owned(),
        title: builder.title,
        blocks: builder.blocks,
    }
}

pub(super) fn anchor_of(heading: &str) -> String {
    let mut plain = String::with_capacity(heading.len());
    let mut rest = heading;

    while let Some(start) = rest.find('[') {
        let (before, found) = rest.split_at(start);
        plain.push_str(before);
        let Some((label, after)) = found.get(1..).and_then(|link| link.split_once("](")) else {
            plain.push_str(found);
            rest = "";
            break;
        };

        plain.push_str(label);
        rest = after.split_once(')').map_or("", |(_, tail)| tail);
    }

    plain.push_str(rest);

    plain
        .chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                Some(ch)
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn link(text: &str) -> Option<(&str, &str)> {
    let (title, rest) = text.strip_prefix('[')?.split_once("](")?;
    let file = rest.strip_suffix(')')?;
    is_page_name(file).then_some((title, file))
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

fn image(line: &str, base: &Path) -> Option<Block> {
    let (alt, source, width) = if let Some(rest) = line.strip_prefix("![") {
        let (alt, rest) = rest.split_once("](")?;
        (alt.to_owned(), rest.strip_suffix(')')?.to_owned(), 0)
    } else if line.starts_with("<img ") && line.ends_with('>') {
        let width = attribute(line, "width").and_then(|w| w.parse().ok());
        (
            attribute(line, "alt").unwrap_or_default().to_owned(),
            attribute(line, "src")?.to_owned(),
            width.unwrap_or(0),
        )
    } else {
        return None;
    };

    Some(match local_file(&source, base) {
        Some(path) => {
            let (natural_width, natural_height) = png_dimensions(&path).unwrap_or_default();

            Block::Image {
                source: format!("file://{}", path.display()),
                alt,
                width,
                natural_width,
                natural_height,
            }
        }
        None => Block::Text { text: alt },
    })
}

/// The URL is not percent-encoded, so nothing may decode to `..` or cut it short.
fn local_file(source: &str, base: &Path) -> Option<PathBuf> {
    let safe = !source.is_empty()
        && !source.starts_with('/')
        && !source.contains([':', '%', '?', '#', '\\'])
        && !source.split('/').any(|part| part == "..");

    safe.then(|| base.join(source))
}

fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    let head = bytes.get(..33)?;
    let (signature, head) = head.split_at(8);
    let (length, head) = head.split_at(4);
    let (kind, size) = head.split_at(4);

    if signature != [137_u8, 80, 78, 71, 13, 10, 26, 10]
        || kind != [73_u8, 72, 68, 82]
        || length != [0_u8, 0, 0, 13]
    {
        return None;
    }

    let width = u32::from_be_bytes(size.get(..4)?.try_into().ok()?);
    let height = u32::from_be_bytes(size.get(4..8)?.try_into().ok()?);

    (width > 0 && height > 0).then_some((width, height))
}

fn attribute<'line>(tag: &'line str, name: &str) -> Option<&'line str> {
    let start = tag.find(&format!(" {name}=\""))? + name.len() + 3;
    let value = tag.get(start..)?;
    value.split_once('"').map(|(value, _)| value)
}

/// Centring has no use in a column and Qt would show the tags.
fn is_paragraph_tag(line: &str) -> bool {
    line == "</p>" || (opens_paragraph(line) && line.ends_with('>') && !line.contains("</p>"))
}

fn opens_paragraph(line: &str) -> bool {
    line.starts_with("<p>") || line.starts_with("<p ")
}

fn without_paragraph_tags(line: &str) -> String {
    let trimmed = line.trim();

    if opens_paragraph(trimmed) && trimmed.ends_with("</p>") {
        let inner = trimmed.split_once('>').map_or("", |(_, rest)| rest);
        inner.trim_end_matches("</p>").to_owned()
    } else {
        line.to_owned()
    }
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
