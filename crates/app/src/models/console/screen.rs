use qtbridge::QListModelBase as _;

use super::{ConsoleModel, LogLine, MAX_LOG_LINES, PromptKind, ScreenBlock, diff};
use crate::hub::term::Changes;
use crate::hub::{ConsoleMsg, ConsoleSession, Prompt};

impl ConsoleModel {
    pub(super) const fn row(&self, position: usize) -> Option<usize> {
        self.log.len().checked_sub(position + 1)
    }

    fn set_line(&mut self, position: usize, line: LogLine) {
        if let Some(index) = self.row(position) {
            self.set(index, line);
        }
    }

    fn insert_line(&mut self, position: usize, line: LogLine) {
        let index = self.log.len() - position.min(self.log.len());
        self.insert(index, line);
    }

    fn remove_line(&mut self, position: usize) {
        if let Some(index) = self.row(position) {
            self.remove(index);
        }
    }

    pub(super) fn handle(&mut self, message: ConsoleMsg) {
        match message {
            ConsoleMsg::Lines(lines) => self.append(lines),
            ConsoleMsg::Screen { session, changes } => self.apply_screen(session, changes),
            ConsoleMsg::Prompt(question) => self.on_prompt(question),
        }
    }

    pub(super) fn answer(&mut self, yes: bool) {
        log::info!(
            "answered {} to {:?}",
            if yes { "yes" } else { "no" },
            self.prompt
        );
        self.prompt.clear();
        self.prompt_core.clear();
        self.hub.answer(yes);
    }

    fn append(&mut self, lines: Vec<String>) {
        for text in lines {
            if self.log.len() >= MAX_LOG_LINES {
                self.drop_oldest();
            }

            self.insert(
                0,
                LogLine {
                    text,
                    ..LogLine::default()
                },
            );
        }
    }

    fn apply_screen(&mut self, session: ConsoleSession, changes: Changes) {
        let block = match self.screen_block {
            Some(block) if block.session == session => block,
            None | Some(_) => ScreenBlock {
                session,
                base: self.log.len(),
                rows: 0,
            },
        };

        let start = (block.base + changes.first).min(self.log.len());
        let end = (block.base + block.rows).clamp(start, self.log.len());
        let hunks = {
            let old: Vec<&str> = self
                .log
                .get(start..end)
                .unwrap_or_default()
                .iter()
                .map(|line| line.raw.as_str())
                .collect();

            diff::hunks(&old, &changes.rows)
        };

        // Bottom up: the rows of the stretches above keep their positions.
        let mut rows = changes.rows;
        for hunk in hunks.iter().rev() {
            let at = start + hunk.old_start;
            for offset in 0..hunk.added {
                let raw = rows
                    .get_mut(hunk.new_start + offset)
                    .map(std::mem::take)
                    .unwrap_or_default();

                let line = self.line_from(raw);
                if offset < hunk.removed {
                    self.set_line(at + offset, line);
                } else {
                    self.insert_line(at + offset, line);
                }
            }

            for _ in hunk.added..hunk.removed {
                self.remove_line(at + hunk.added);
            }
        }

        self.screen_block = Some(ScreenBlock {
            rows: changes.total,
            ..block
        });

        while self.log.len() > MAX_LOG_LINES && !self.log.is_empty() {
            self.drop_oldest();
        }
    }

    fn drop_oldest(&mut self) {
        self.pop();
        if let Some(shown) = self.screen_block.as_mut() {
            shown.base = shown.base.saturating_sub(1);
        }
    }

    fn line_from(&self, raw: String) -> LogLine {
        LogLine {
            text: crate::hub::strip_ansi(&raw),
            html: crate::hub::pty::ansi_to_html(&raw, &self.palette),
            raw,
        }
    }

    pub(super) fn recolour(&mut self, palette: Vec<String>) {
        if palette == self.palette {
            return;
        }

        log::debug!("console palette changed: {palette:?}");
        self.palette = palette;

        for position in 0..self.log.len() {
            let Some(line) = self.log.get(position).filter(|line| !line.raw.is_empty()) else {
                continue;
            };

            let recoloured = LogLine {
                html: crate::hub::pty::ansi_to_html(&line.raw, &self.palette),
                ..line.clone()
            };

            self.set_line(position, recoloured);
        }
    }

    fn on_prompt(&mut self, question: Option<Prompt>) {
        let Some(question) = question else {
            self.prompt_kind = PromptKind::YesNo;
            self.prompt.clear();
            self.prompt_core.clear();
            return;
        };

        self.prompt_kind = if question.password {
            PromptKind::Password
        } else {
            PromptKind::YesNo
        };
        self.prompt = question.text;
        self.prompt_core = question.core;
    }

    pub(super) fn log_lines(&self) -> String {
        self.log
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
