use super::kind::InputKind;
use super::parse::Attrs;

pub struct Urls {
    pub web: Option<String>,
    pub commit: Option<String>,
    pub remote: Option<String>,
}

impl Urls {
    const NONE: Self = Self {
        web: None,
        commit: None,
        remote: None,
    };

    pub fn for_node(kind: &InputKind, locked: &Attrs, original: &Attrs) -> Self {
        let rev = locked.rev.as_deref();

        match kind {
            InputKind::Github => Self::forge(locked, rev, ("github.com", "commit/", ".git")),
            InputKind::Gitlab => Self::forge(locked, rev, ("gitlab.com", "-/commit/", ".git")),
            InputKind::Sourcehut => Self::forge(locked, rev, ("git.sr.ht", "commit/", "")),
            InputKind::Git => Self::git(
                original
                    .url
                    .as_deref()
                    .or(locked.url.as_deref())
                    .unwrap_or_default(),
            ),
            InputKind::Tarball | InputKind::File => {
                Self::tarball(original.url.as_deref().unwrap_or_default(), rev)
            }
            InputKind::Path | InputKind::Indirect | InputKind::Follows | InputKind::Other(_) => {
                Self::NONE
            }
        }
    }

    fn forge(
        locked: &Attrs,
        rev: Option<&str>,
        (default_host, commit_infix, git_suffix): (&str, &str, &str),
    ) -> Self {
        let web = format!(
            "https://{}/{}/{}",
            locked.host.as_deref().unwrap_or(default_host),
            unescape_slashes(locked.owner.as_deref().unwrap_or_default()),
            locked.repo.as_deref().unwrap_or_default()
        );

        Self {
            commit: rev.map(|rev| format!("{web}/{commit_infix}{rev}")),
            remote: Some(format!("{web}{git_suffix}")),
            web: Some(web),
        }
    }

    fn tarball(url: &str, rev: Option<&str>) -> Self {
        let remote = url.starts_with("https://") || url.starts_with("http://");
        let from_nixos = url.contains("nixos.org/");

        Self {
            web: from_nixos.then(|| "https://github.com/NixOS/nixpkgs".to_owned()),
            commit: rev
                .filter(|_| from_nixos)
                .map(|rev| format!("https://github.com/NixOS/nixpkgs/commit/{rev}")),
            remote: remote.then(|| url.to_owned()),
        }
    }

    fn git(url: &str) -> Self {
        let url = url.strip_prefix("git+").unwrap_or(url);
        let remote = ["https://", "http://", "ssh://", "git://"]
            .iter()
            .any(|scheme| url.starts_with(scheme));

        Self {
            web: url
                .starts_with("https://")
                .then(|| url.trim_end_matches(".git").to_owned()),
            commit: None,
            remote: remote.then(|| url.to_owned()),
        }
    }
}

/// A GitLab subgroup's slash is written `%2F` in references but plain in web and git addresses.
fn unescape_slashes(owner: &str) -> String {
    owner.replace("%2F", "/").replace("%2f", "/")
}

#[cfg(test)]
mod tests;
