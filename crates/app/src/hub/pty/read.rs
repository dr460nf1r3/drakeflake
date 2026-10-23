use std::fs::File;
use std::io::Read as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

#[cfg(test)]
mod tests;

const READ_BUFFER: usize = 8192;

/// How long the reader sleeps when the terminal has nothing to read. It
/// polls, so it can be stopped while a grandchild still holds the terminal.
const READ_IDLE: Duration = Duration::from_millis(10);

pub(super) enum Output {
    Chunk(String),
    Closed,
}

pub(super) fn read_output(mut master: File, sender: mpsc::Sender<Output>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);

    if let Err(err) = rustix::fs::fcntl_setfl(&master, rustix::fs::OFlags::NONBLOCK) {
        log::warn!("cannot poll the terminal, the reader may outlive the program: {err}");
    }

    let spawned = std::thread::Builder::new()
        .name("df-pty".to_owned())
        .spawn(move || {
            let mut buffer = [0_u8; READ_BUFFER];
            let mut undecoded: Vec<u8> = Vec::new();

            while !stopped.load(Ordering::SeqCst) {
                let read = match master.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => read,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(READ_IDLE);
                        continue;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(err) => {
                        log::trace!("terminal closed: {err}");
                        break;
                    }
                };

                log::trace!("read {read} bytes from the terminal");
                undecoded.extend_from_slice(buffer.get(..read).unwrap_or_default());

                if let Some(text) = take_decoded(&mut undecoded)
                    && sender.send(Output::Chunk(text)).is_err()
                {
                    return;
                }
            }

            if !undecoded.is_empty() {
                let rest = String::from_utf8_lossy(&undecoded).into_owned();
                sender.send(Output::Chunk(rest)).ok();
            }

            sender.send(Output::Closed).ok();
        });

    if let Err(err) = spawned {
        log::warn!("cannot read the program's output: {err}");
    }

    stop
}

fn take_decoded(bytes: &mut Vec<u8>) -> Option<String> {
    let valid = match std::str::from_utf8(bytes) {
        Ok(text) => text.len(),
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => bytes.len(),
    };

    let chunk: Vec<u8> = bytes.drain(..valid).collect();
    (!chunk.is_empty()).then(|| String::from_utf8_lossy(&chunk).into_owned())
}
