mod style;
#[cfg(test)]
mod tests;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermDamage};
use alacritty_terminal::vte::ansi::Processor;
use style::Style;

#[derive(Debug, PartialEq, Eq)]
pub struct Changes {
    pub first: usize,
    pub rows: Vec<String>,
    pub total: usize,
}

pub struct Emulator {
    term: Term<VoidListener>,
    parser: Processor,
    lines: usize,
    dirty: bool,
}

struct Size {
    columns: usize,
    lines: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

impl Emulator {
    pub fn new(columns: usize, lines: usize) -> Self {
        let size = Size {
            columns: columns.max(1),
            lines: lines.max(1),
        };

        let config = Config {
            scrolling_history: 0,
            ..Default::default()
        };

        let mut emulator = Self {
            term: Term::new(config, &size, VoidListener),
            parser: Processor::new(),
            lines: size.lines,
            dirty: false,
        };

        emulator.term.reset_damage();
        emulator
    }

    pub fn feed(&mut self, text: &str) {
        self.parser.advance(&mut self.term, text.as_bytes());
        self.dirty = true;
    }

    pub fn resize(&mut self, columns: usize, lines: usize) {
        let size = Size {
            columns: columns.max(1),
            lines: lines.max(1),
        };

        if size.columns == self.term.columns() && size.lines == self.term.screen_lines() {
            return;
        }

        self.term.resize(size);
        self.lines = self.term.screen_lines();
        self.dirty = true;
    }

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn take_changes(&mut self) -> Option<Changes> {
        if !self.dirty {
            return None;
        }

        self.dirty = false;
        let mut range: Option<(usize, usize)> = None;

        match self.term.damage() {
            TermDamage::Full => range = Some((0, self.lines.saturating_sub(1))),
            TermDamage::Partial(damaged) => {
                for bounds in damaged {
                    let line = bounds.line.min(self.lines.saturating_sub(1));
                    range = Some(match range {
                        Some((first, end)) => (first.min(line), end.max(line)),
                        None => (line, line),
                    });
                }
            }
        }

        self.term.reset_damage();

        let total = self.content_lines();
        let first = range.map_or(0, |(first, _)| first.min(total));
        let rows = (first..total).map(|line| self.render(line)).collect();

        Some(Changes { first, rows, total })
    }

    fn content_lines(&self) -> usize {
        (0..self.lines)
            .rfind(|line| !self.line_is_blank(*line))
            .map_or(0, |last| last + 1)
    }

    pub fn rows(&self) -> Vec<String> {
        (0..self.content_lines())
            .map(|line| self.render(line))
            .collect()
    }

    pub fn unfinished_line(&self) -> Option<String> {
        let point = self.term.grid().cursor.point;
        let cursor = usize::try_from(point.line.0).ok()?;

        if cursor >= self.lines || self.line_is_blank(cursor) {
            return None;
        }

        if (cursor + 1..self.lines).any(|line| !self.line_is_blank(line)) {
            return None;
        }

        Some(self.render(cursor))
    }

    fn line_is_blank(&self, line: usize) -> bool {
        let row = &self.term.grid()[Line(i32::try_from(line).unwrap_or(0_i32))];

        (0..self.term.columns()).all(|column| {
            let cell = &row[Column(column)];
            cell.c == ' ' || cell.c == '\0'
        })
    }

    fn render(&self, line: usize) -> String {
        let row = &self.term.grid()[Line(i32::try_from(line).unwrap_or(0_i32))];
        let mut out = String::new();
        let mut style = Style::default();

        for column in 0..self.term.columns() {
            let cell = &row[Column(column)];

            if cell.flags.contains(Flags::WIDE_CHAR_SPACER)
                || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let next = Style::of(cell);

            if next != style {
                if style != Style::default() {
                    out.push_str("\x1b[m");
                }

                next.push_sgr(&mut out);
                style = next;
            }

            out.push(cell.c);
        }

        while out.ends_with(' ') {
            out.pop();
        }

        if let Some(start) = out.rfind('\x1b')
            && let Some(tail) = out.get(start..)
            && let Some(params) = tail
                .strip_prefix("\x1b[")
                .and_then(|tail| tail.strip_suffix('m'))
            && params
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b';')
        {
            out.truncate(start);
        }

        out
    }
}
