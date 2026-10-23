use drakeflake_render::OptionName;
use rnix::ast::{self, HasEntry};
use rowan::ast::AstNode as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    pub line: usize,
    pub column: usize,
}

struct Walk<'a> {
    text: &'a str,
    target: &'a [&'a str],
    sites: &'a mut Vec<Site>,
}

impl Walk<'_> {
    fn expr(&mut self, expr: &ast::Expr, prefix: &[String]) {
        match expr {
            ast::Expr::Lambda(lambda) => self.opt(lambda.body(), prefix),
            ast::Expr::AttrSet(set) => self.entries(set, prefix),
            ast::Expr::Paren(paren) => self.opt(paren.expr(), prefix),
            ast::Expr::LetIn(let_in) => self.opt(let_in.body(), prefix),
            ast::Expr::With(with) => self.opt(with.body(), prefix),
            ast::Expr::IfElse(if_else) => {
                self.opt(if_else.body(), prefix);
                self.opt(if_else.else_body(), prefix);
            }
            ast::Expr::Apply(apply) => self.opt(apply.argument(), prefix),
            ast::Expr::List(list) => {
                for item in list.items() {
                    self.expr(&item, prefix);
                }
            }
            ast::Expr::Assert(_)
            | ast::Expr::Error(_)
            | ast::Expr::Select(_)
            | ast::Expr::Str(_)
            | ast::Expr::PathAbs(_)
            | ast::Expr::PathRel(_)
            | ast::Expr::PathHome(_)
            | ast::Expr::PathSearch(_)
            | ast::Expr::Literal(_)
            | ast::Expr::LegacyLet(_)
            | ast::Expr::BinOp(_)
            | ast::Expr::Root(_)
            | ast::Expr::UnaryOp(_)
            | ast::Expr::Ident(_)
            | ast::Expr::HasAttr(_)
            | ast::Expr::CurPos(_) => {}
        }
    }

    fn opt(&mut self, expr: Option<ast::Expr>, prefix: &[String]) {
        if let Some(expr) = expr {
            self.expr(&expr, prefix);
        }
    }

    fn entries(&mut self, set: &impl HasEntry, prefix: &[String]) {
        for entry in set.entries() {
            match entry {
                ast::Entry::AttrpathValue(binding) => self.binding(&binding, prefix),
                ast::Entry::Inherit(inherit) => {
                    for attr in inherit.attrs() {
                        if let Some(name) = attr_name(&attr) {
                            let mut path = prefix.to_vec();
                            path.push(name);
                            self.record(&path, attr.syntax());
                        }
                    }
                }
            }
        }
    }

    fn binding(&mut self, binding: &ast::AttrpathValue, prefix: &[String]) {
        let Some(attrpath) = binding.attrpath() else {
            return;
        };

        let mut path = prefix.to_vec();

        for attr in attrpath.attrs() {
            let Some(name) = attr_name(&attr) else {
                return;
            };

            path.push(name);
        }

        if prefix.is_empty() && path.len() > 1 && path.first().is_some_and(|key| key == "config") {
            path.remove(0);
        }

        self.record(&path, attrpath.syntax());

        let Some(value) = binding.value() else {
            return;
        };

        if prefix.is_empty() && path.len() == 1 && path.first().is_some_and(|key| key == "config") {
            self.expr(&value, &[]);
        }

        if self.leads_to_target(&path) {
            self.expr(&value, &path);
        }
    }

    fn record(&mut self, path: &[String], node: &rnix::SyntaxNode) {
        let defines = path.len() >= self.target.len()
            && self
                .target
                .iter()
                .zip(path)
                .all(|(wanted, name)| *wanted == name);

        if defines {
            self.sites
                .push(site_at(self.text, usize::from(node.text_range().start())));
        }
    }

    fn leads_to_target(&self, path: &[String]) -> bool {
        path.len() < self.target.len()
            && path
                .iter()
                .zip(self.target)
                .all(|(name, wanted)| name == wanted)
    }
}

#[must_use = "parsing the file has no other effect"]
pub fn definition_sites(text: &str, option: &OptionName) -> Vec<Site> {
    let target: Vec<&str> = option.split('.').collect();
    let mut sites = Vec::new();

    if let Some(expr) = rnix::Root::parse(text).tree().expr() {
        let mut walk = Walk {
            text,
            target: &target,
            sites: &mut sites,
        };

        walk.expr(&expr, &[]);
    }

    sites.sort_by_key(|site| (site.line, site.column));
    sites.dedup();
    sites
}

#[must_use = "parsing the file has no other effect"]
pub fn import_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();

    if let Some(expr) = rnix::Root::parse(text).tree().expr() {
        collect_imports(&expr, &mut paths);
    }

    paths
}

#[must_use = "the attribute name is the only result"]
pub fn attr_name(attr: &ast::Attr) -> Option<String> {
    match attr {
        ast::Attr::Ident(ident) => Some(ident.syntax().text().to_string()),
        ast::Attr::Str(string) => match string.normalized_parts().as_slice() {
            [ast::InterpolPart::Literal(text)] => Some(text.clone()),
            [] => Some(String::new()),
            _ => None,
        },
        ast::Attr::Dynamic(_) => None,
    }
}

#[must_use = "the site is only computed"]
pub fn site_at(text: &str, offset: usize) -> Site {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;

    Site { line, column }
}

fn collect_imports(expr: &ast::Expr, paths: &mut Vec<String>) {
    let inner = match expr {
        ast::Expr::Lambda(lambda) => lambda.body(),
        ast::Expr::Paren(paren) => paren.expr(),
        ast::Expr::LetIn(let_in) => let_in.body(),
        ast::Expr::AttrSet(set) => {
            for entry in set.entries() {
                if let ast::Entry::AttrpathValue(binding) = entry
                    && binding
                        .attrpath()
                        .is_some_and(|attrpath| attrpath.syntax().text() == "imports")
                    && let Some(value) = binding.value()
                {
                    list_paths(&value, paths);
                }
            }

            None
        }
        ast::Expr::Apply(_)
        | ast::Expr::Assert(_)
        | ast::Expr::Error(_)
        | ast::Expr::IfElse(_)
        | ast::Expr::Select(_)
        | ast::Expr::Str(_)
        | ast::Expr::PathAbs(_)
        | ast::Expr::PathRel(_)
        | ast::Expr::PathHome(_)
        | ast::Expr::PathSearch(_)
        | ast::Expr::Literal(_)
        | ast::Expr::LegacyLet(_)
        | ast::Expr::List(_)
        | ast::Expr::BinOp(_)
        | ast::Expr::Root(_)
        | ast::Expr::UnaryOp(_)
        | ast::Expr::Ident(_)
        | ast::Expr::With(_)
        | ast::Expr::HasAttr(_)
        | ast::Expr::CurPos(_) => None,
    };

    if let Some(inner) = inner {
        collect_imports(&inner, paths);
    }
}

fn list_paths(expr: &ast::Expr, paths: &mut Vec<String>) {
    match expr {
        ast::Expr::List(list) => {
            for item in list.items() {
                list_paths(&item, paths);
            }
        }
        ast::Expr::PathRel(_) | ast::Expr::PathAbs(_) | ast::Expr::PathHome(_) => {
            paths.push(expr.syntax().text().to_string());
        }
        ast::Expr::Paren(paren) => {
            if let Some(inner) = paren.expr() {
                list_paths(&inner, paths);
            }
        }
        ast::Expr::BinOp(op) => {
            for side in [op.lhs(), op.rhs()].into_iter().flatten() {
                list_paths(&side, paths);
            }
        }
        ast::Expr::Apply(apply) => {
            if let Some(argument) = apply.argument() {
                list_paths(&argument, paths);
            }
        }
        ast::Expr::Assert(_)
        | ast::Expr::Error(_)
        | ast::Expr::IfElse(_)
        | ast::Expr::Select(_)
        | ast::Expr::Str(_)
        | ast::Expr::PathSearch(_)
        | ast::Expr::Literal(_)
        | ast::Expr::Lambda(_)
        | ast::Expr::LegacyLet(_)
        | ast::Expr::LetIn(_)
        | ast::Expr::Root(_)
        | ast::Expr::AttrSet(_)
        | ast::Expr::UnaryOp(_)
        | ast::Expr::Ident(_)
        | ast::Expr::With(_)
        | ast::Expr::HasAttr(_)
        | ast::Expr::CurPos(_) => {}
    }
}

#[cfg(test)]
mod tests;
