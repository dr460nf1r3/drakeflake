use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Plain,
    Color,
}

fn paint(style: Style, code: &str, text: &str) -> String {
    match style {
        Style::Plain => text.to_owned(),
        Style::Color => format!("\x1b[{code}m{text}\x1b[0m"),
    }
}

const fn level_color(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => "1;31",
        log::Level::Warn => "33",
        log::Level::Info => "32",
        log::Level::Debug => "34",
        log::Level::Trace => "35",
    }
}

pub fn format_record(
    time: SystemTime,
    level: log::Level,
    target: &str,
    message: &str,
    style: Style,
) -> String {
    let mut lines = message.trim_end().lines();
    let mut out = format!(
        "{} {} {}: {}\n",
        paint(style, "2", &timestamp(time)),
        paint(style, level_color(level), &format!("{level:<5}")),
        paint(style, "36", target),
        lines.next().unwrap_or_default()
    );

    for line in lines {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }

    out
}

/// UTC ISO 8601 with milliseconds, computed by hand to avoid a time zone
/// database dependency.
pub fn timestamp(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since.as_secs();
    let days = i64::try_from(secs / 86_400).unwrap_or_default();
    let rest = secs % 86_400;
    let (year, month, day) = civil_from_days(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60,
        since.subsec_millis()
    )
}

const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}
