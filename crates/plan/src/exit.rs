//! The helper never returns pkexec's own codes, so a code says who ended the run.

pub const USAGE: u8 = 2;
pub const REFUSED: u8 = 3;
pub const IO: u8 = 4;
pub const TOOL: u8 = 5;
pub const PKEXEC_NOT_AUTHORIZED: u8 = 126;
pub const PKEXEC_DISMISSED: u8 = 127;
