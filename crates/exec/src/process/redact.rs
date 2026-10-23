use std::borrow::Cow;

const HIDDEN: &str = "***";
const HIDDEN_USERINFO: &str = "***@";

/// Lock URLs with tokens must never reach the log.
#[must_use = "the redacted text is returned, the input is unchanged"]
pub fn redact_urls(text: &str) -> String {
    text.split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> Cow<'_, str> {
    let Some((scheme, rest)) = token.split_once("://") else {
        return Cow::Borrowed(token);
    };

    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at_checked(end).unwrap_or((rest, ""));
    let (userinfo, host) = authority
        .rsplit_once('@')
        .map_or(("", authority), |(_, host)| (HIDDEN_USERINFO, host));

    Cow::Owned(format!("{scheme}://{userinfo}{host}{}", redact_query(tail)))
}

fn redact_query(tail: &str) -> String {
    let Some((path, query)) = tail.split_once('?') else {
        return tail.to_owned();
    };

    let (query, fragment) = query
        .split_once('#')
        .map_or((query, None), |(query, fragment)| (query, Some(fragment)));

    let pairs = query
        .split('&')
        .map(|pair| {
            pair.split_once('=')
                .map_or_else(|| pair.to_owned(), |(key, _)| format!("{key}={HIDDEN}"))
        })
        .collect::<Vec<_>>()
        .join("&");

    fragment.map_or_else(
        || format!("{path}?{pairs}"),
        |fragment| format!("{path}?{pairs}#{fragment}"),
    )
}

#[cfg(test)]
mod tests;
