use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt as _;
use std::path::{Path, PathBuf};

const MOUNT_TABLE: &str = "/proc/self/mountinfo";

pub fn mount_points() -> Result<Vec<PathBuf>, String> {
    fs::read_to_string(MOUNT_TABLE)
        .map(|text| parse_mountinfo(&text))
        .map_err(|err| format!("cannot read {MOUNT_TABLE}: {err}"))
}

#[must_use = "only parses the text"]
pub fn parse_mountinfo(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| line.split(' ').nth(4))
        .map(mount_point)
        .collect()
}

#[must_use = "only parses the text"]
pub fn mount_source(text: &str, point: &Path) -> Option<(String, String)> {
    text.lines()
        .filter(|line| {
            line.split(' ')
                .nth(4)
                .is_some_and(|field| mount_point(field).as_path() == point)
        })
        .rev()
        .find_map(|line| {
            let (_, rest) = line.split_once(" - ")?;
            let mut fields = rest.split(' ');
            Some((lossy(fields.next()?), lossy(fields.next()?)))
        })
}

#[must_use = "only reads the mount table"]
pub fn root_file_system() -> Option<String> {
    let text = fs::read_to_string(MOUNT_TABLE).ok()?;
    mount_source(&text, Path::new("/")).map(|(kind, _)| kind)
}

fn mount_point(field: &str) -> PathBuf {
    PathBuf::from(OsString::from_vec(unescape(field)))
}

fn lossy(field: &str) -> String {
    String::from_utf8_lossy(&unescape(field)).into_owned()
}

fn unescape(field: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(field.len());
    let mut rest = field.as_bytes();

    while let Some((&byte, tail)) = rest.split_first() {
        let code = tail
            .first_chunk::<3>()
            .copied()
            .filter(|_| byte == b'\\')
            .and_then(octal);

        if let Some(code) = code {
            out.push(code);
            rest = tail.get(3..).unwrap_or_default();
        } else {
            out.push(byte);
            rest = tail;
        }
    }

    out
}

/// Digit by digit because `from_str_radix` also accepts a leading `+`.
fn octal(digits: [u8; 3]) -> Option<u8> {
    digits.into_iter().try_fold(0_u8, |code, digit| {
        let value = digit.checked_sub(b'0').filter(|value| *value < 8)?;
        code.checked_mul(8)?.checked_add(value)
    })
}
