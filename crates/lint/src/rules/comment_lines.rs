use crate::finding::Finding;
use crate::source::{Kind, Source, Unit};

const RULE: &str = "max-comment-lines";
const BODY: usize = 3;
const DOC_DESCRIPTION: usize = 6;
const OTHER: usize = 4;

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for unit in source.units() {
        match unit.kind() {
            Kind::InnerDoc => {}
            Kind::Doc => {
                let count = description_lines(&unit);
                if count > DOC_DESCRIPTION {
                    findings.push(
                        Finding::new(
                            unit.start(),
                            RULE,
                            format!("Doc description has {count} lines, more than {DOC_DESCRIPTION}."),
                            "Doc comments are read in hover popups. Past a few lines they are skipped.",
                        )
                        .fix("Keep the contract (what, preconditions, side effects) and move background and history to the module docs or the docs folder."),
                    );
                }
            }
            Kind::Line | Kind::Block => {
                let count = unit.height();
                let in_body = source.in_function_body(unit.first().node);
                let max = if in_body { BODY } else { OTHER };
                if count <= max {
                    continue;
                }

                let finding = if in_body {
                    Finding::new(
                        unit.start(),
                        RULE,
                        format!("Comment inside a function body has {count} lines, more than {max}."),
                        "A long explanation inside a function means the code is not saying enough itself, or the text is padding.",
                    )
                    .fix("Cut it to the one non-obvious reason. Extract a well-named function for the rest, and move background to the docs or the commit message.")
                } else {
                    Finding::new(
                        unit.start(),
                        RULE,
                        format!("Comment has {count} lines, more than {max}."),
                        "Long comments are skipped by readers and drift from the code.",
                    )
                    .fix("Shorten it to what the code cannot say. If it describes an item, make it a doc comment.")
                };

                findings.push(finding);
            }
        }
    }
}

fn description_lines(unit: &Unit<'_, '_>) -> usize {
    unit.lines()
        .take_while(|line| {
            let text = line.text.trim_start();
            !(text.starts_with("# ") || text.starts_with('@') || text.starts_with("```"))
        })
        .filter(|line| !line.text.trim().is_empty())
        .count()
}
