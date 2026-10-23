use std::path::Path;

use super::*;

#[test]
fn echoes_the_line_with_paths_as_text() {
    let args = [
        arg("build"),
        arg(Path::new("/tmp/out")),
        arg(String::from("x")),
    ];

    assert_eq!(echo(OsStr::new("nh"), &args), "$ nh build /tmp/out x");
    assert_eq!(echo(OsStr::new("nh"), &[]), "$ nh");
}

#[cfg(unix)]
#[test]
fn an_argument_keeps_bytes_that_are_not_utf8() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let odd = OsString::from_vec(vec![b'a', 0xff]);

    assert_eq!(arg(&odd).as_bytes(), [b'a', 0xff]);
    assert_eq!(echo(OsStr::new("x"), &[odd]), "$ x a\u{fffd}");
}
