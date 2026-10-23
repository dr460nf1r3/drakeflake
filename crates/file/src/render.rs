use std::collections::BTreeMap;
use std::path::PathBuf;

use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_plan::{FILES_PREFIX, HEADER, ManagedState, Rendered, RenderedFile, STATE_PREFIX};
use drakeflake_render::{HostName, NixValue, render_value, split_option_name};
use drakeflake_validate::parse_package;
use log::debug;

use crate::error::FileError;
use crate::layout::{PACKAGES_PATH, file_for, packages_file};
use crate::module::{self, Body, Setting};
use crate::option::{ManagedOptionExt as _, RenderedValue};
use crate::paths::{DEFAULT_FOLDER, FOLDER_ENTRY};
use crate::state::{IMPORT_MARKER, IMPORT_MARKER_COMMENT};

/// A guard against pathological input, not a practical limit, since a real configuration is a few kilobytes.
const MAX_STATE_LINE: usize = 4 * 1024 * 1024;

struct Placed {
    file: Option<String>,
    host: Option<HostName>,
    setting: Setting,
}

pub trait ManagedStateExt {
    fn render(&self) -> Result<String, Vec<FileError>>;

    fn render_layout(
        &self,
        layout: &LayoutConfig,
        folder: &str,
    ) -> Result<Rendered, Vec<FileError>>;
}

impl ManagedStateExt for ManagedState {
    fn render(&self) -> Result<String, Vec<FileError>> {
        let single = LayoutConfig {
            mode: LayoutMode::Single,
            rules: Vec::new(),
        };

        self.render_layout(&single, DEFAULT_FOLDER)
            .map(|rendered| rendered.entry)
    }

    fn render_layout(
        &self,
        layout: &LayoutConfig,
        folder: &str,
    ) -> Result<Rendered, Vec<FileError>> {
        let mut entry = Body {
            preamble: vec![IMPORT_MARKER_COMMENT.to_owned(), IMPORT_MARKER.to_owned()],
            ..Body::default()
        };

        let mut files: BTreeMap<String, Body> = BTreeMap::new();

        for placed in assignments(self, layout)? {
            let body = placed.file.map_or(&mut entry, |file| {
                let path = if folder.is_empty() {
                    file
                } else {
                    format!("{folder}/{file}")
                };

                files.entry(path).or_default()
            });

            match placed.host {
                Some(host) => body.hosts.entry(host).or_default().push(placed.setting),
                None => body.shared.push(placed.setting),
            }
        }

        entry.imports = files.keys().cloned().collect();
        let entry_header = entry_header(self, &entry.imports)?;
        let entry_name = if folder.is_empty() {
            FOLDER_ENTRY.to_owned()
        } else {
            format!("{folder}.nix")
        };

        let part_header = format!("{HEADER}# Part of the settings imported by {entry_name}.\n");

        debug!(
            "rendered {} layout: {} entry settings, {} part files",
            layout.mode,
            entry.shared.len() + entry.hosts.values().map(Vec::len).sum::<usize>(),
            files.len()
        );

        Ok(Rendered {
            entry: module::module_text(&entry_header, &entry),
            parts: files
                .into_iter()
                .map(|(path, body)| RenderedFile {
                    path: PathBuf::from(path),
                    text: module::module_text(&part_header, &body),
                })
                .collect(),
        })
    }
}

fn packages_setting(
    state: &ManagedState,
    layout: &LayoutConfig,
    errors: &mut Vec<FileError>,
) -> Placed {
    let mut packages = Vec::with_capacity(state.packages.len());

    for attr in &state.packages {
        match parse_package(attr) {
            Ok(expr) => packages.push(render_value(&NixValue::Raw(expr))),
            Err(source) => errors.push(FileError::Package {
                attr: attr.clone(),
                source,
            }),
        }
    }

    Placed {
        file: packages_file(layout),
        host: None,
        setting: Setting {
            path: split_option_name(PACKAGES_PATH),
            value: RenderedValue {
                text: format!("[ {} ]", packages.join(" ")),
                verbatim: false,
            },
        },
    }
}

fn assignments(state: &ManagedState, layout: &LayoutConfig) -> Result<Vec<Placed>, Vec<FileError>> {
    let mut out = Vec::new();
    let mut errors = Vec::new();

    if !state.packages.is_empty() {
        out.push(packages_setting(state, layout, &mut errors));
    }

    for (name, option) in &state.options {
        match option.render(name) {
            Ok(value) => out.push(Placed {
                file: file_for(layout, name),
                host: option.host.clone(),
                setting: Setting {
                    path: split_option_name(name),
                    value,
                },
            }),
            Err(source) => errors.push(FileError::Option {
                name: name.clone(),
                source: Box::new(source),
            }),
        }
    }

    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// The state line must stay small enough for line-oriented tools.
fn entry_header(state: &ManagedState, parts: &[String]) -> Result<String, Vec<FileError>> {
    let encoded = serde_json::to_string(state).map_err(|err| vec![FileError::Encode(err)])?;
    let listed = serde_json::to_string(parts).map_err(|err| vec![FileError::Encode(err)])?;

    if encoded.len() > MAX_STATE_LINE {
        return Err(vec![FileError::StateTooLarge(encoded.len())]);
    }

    Ok(format!(
        "{HEADER}{STATE_PREFIX}{encoded}\n{FILES_PREFIX}{listed}\n"
    ))
}
