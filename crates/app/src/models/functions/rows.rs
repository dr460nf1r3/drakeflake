use drakeflake_functions::{Function, Position, Source, blocks};
use qtbridge::QModelItem;
use serde_json::{Value, json};

use super::FunctionModel;
use crate::models::one_line;

const SUMMARY_WIDTH: usize = 200;

#[derive(Clone, Default, PartialEq, Eq, QModelItem, serde::Serialize)]
pub struct FunctionRow {
    pub path: String,
    pub summary: String,
    pub signature: String,
    pub builtin: bool,
}

impl FunctionModel {
    pub(super) fn row(&self, position: usize) -> Option<FunctionRow> {
        let function = self.index.get(position)?;

        Some(FunctionRow {
            path: function.path.clone(),
            summary: one_line(self.index.summary(position), SUMMARY_WIDTH),
            signature: function.signature.clone(),
            builtin: function.builtin,
        })
    }

    pub(super) fn details_json(&self, path: &str) -> Value {
        let Some(position) = self.index.position(path) else {
            return json!({ "known": false, "path": path });
        };

        let Some(function) = self.index.get(position) else {
            return json!({ "known": false, "path": path });
        };

        let shape = self.index.shape(position);
        let rev = &self.index.source().rev;

        json!({
            "known": true,
            "path": function.path,
            "namespace": function.namespace(),
            "summary": self.index.summary(position),
            "blocks": blocks(&function.doc, &function.signature),
            "doc_from": function.doc_from,
            "signature": function.signature,
            "takes": shape.takes,
            "returns": shape.returns,
            "args": function.args,
            "builtin": function.builtin,
            "experimental": function.experimental,
            "impure": function.impure,
            "aliases": function.aliases,
            "attr": position_json(function.attr.as_ref(), rev),
            "lambda": position_json(function.lambda.as_ref(), rev),
            "code": function.code,
            "manual": manual_url(function),
        })
    }
}

pub fn source_json(source: &Source) -> Value {
    let url = if source.rev.is_empty() {
        String::new()
    } else {
        format!("https://github.com/NixOS/nixpkgs/tree/{}", source.rev)
    };

    json!({
        "rev": source.rev,
        "date": source.date,
        "nix": source.nix,
        "url": url,
    })
}

fn position_json(position: Option<&Position>, rev: &str) -> Value {
    position.map_or(Value::Null, |position| {
        json!({
            "file": position.file,
            "line": position.line,
            "url": position.url(rev).unwrap_or_default(),
        })
    })
}

fn manual_url(function: &Function) -> String {
    let path = &function.path;

    match path.strip_prefix("builtins.") {
        Some(name) => {
            format!("https://nix.dev/manual/nix/latest/language/builtins.html#builtins-{name}")
        }
        None if path.starts_with("lib.") => {
            format!("https://nixos.org/manual/nixpkgs/unstable/#function-library-{path}")
        }
        None => String::new(),
    }
}
