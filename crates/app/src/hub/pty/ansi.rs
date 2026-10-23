const COLORS: [&str; 8] = [
    "#7f8c8d", "#da4453", "#27ae60", "#c9a227", "#3daee9", "#9b59b6", "#16a085", "#95a5a6",
];

#[derive(Default, Clone)]
struct Style {
    color: Option<String>,
    bold: bool,
}

impl Style {
    fn apply(&mut self, params: &str, palette: &[String]) {
        let pick = |index: u32| {
            let index = usize::try_from(index).map_or(0, |index| index % COLORS.len());
            palette
                .get(index)
                .filter(|color| is_safe_color(color))
                .cloned()
                .or_else(|| COLORS.get(index).map(|color| (*color).to_owned()))
        };

        let mut codes = params
            .split(';')
            .map(|code| code.parse::<u32>().unwrap_or(0));

        while let Some(code) = codes.next() {
            match code {
                0 => *self = Self::default(),
                1 => self.bold = true,
                22 => self.bold = false,
                39 => self.color = None,
                30..=37 | 90..=97 => self.color = pick(code % 10),
                38 => {
                    let basic = match codes.next() {
                        Some(5) => codes.next().map(|index| index % 8),
                        Some(2) => {
                            codes.nth(2);
                            Some(7)
                        }
                        _ => None,
                    };

                    self.color = basic.and_then(pick);
                }
                _ => {}
            }
        }
    }
}

pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        match chars.peek() {
            Some('[') => {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(next) = chars.next() {
                    if next == '\u{7}' {
                        break;
                    }

                    if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            Some(' '..='/') => {
                for next in chars.by_ref() {
                    if !(' '..='/').contains(&next) {
                        break;
                    }
                }
            }
            _ => {
                chars.next();
            }
        }
    }

    out
}

pub fn ansi_to_html(text: &str, palette: &[String]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut style = Style::default();
    let mut open = Style::default();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut params = String::new();
            let mut command = ' ';
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    command = next;
                    break;
                }

                params.push(next);
            }

            if command == 'm' {
                style.apply(&params, palette);
                close(&mut out, &open);
                if let Some(color) = &style.color {
                    out.push_str("<font color=\"");
                    out.push_str(color);
                    out.push_str("\">");
                }

                if style.bold {
                    out.push_str("<b>");
                }

                open = style.clone();
            }

            continue;
        }

        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            ' ' => out.push_str("&nbsp;"),
            _ if ch.is_control() => {}
            _ => out.push(ch),
        }
    }

    close(&mut out, &open);
    out
}

fn close(out: &mut String, open: &Style) {
    if open.bold {
        out.push_str("</b>");
    }

    if open.color.is_some() {
        out.push_str("</font>");
    }
}

fn is_safe_color(color: &str) -> bool {
    let digits = color.strip_prefix('#').unwrap_or("");
    let len = digits.len();
    (len == 3 || len == 6) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
}
