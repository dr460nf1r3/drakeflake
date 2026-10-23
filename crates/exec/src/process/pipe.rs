use std::io::Read;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

const CHUNK_BYTES: usize = 16 * 1024;

pub struct Collector {
    buffer: Arc<Mutex<Vec<u8>>>,
    closed: mpsc::Receiver<()>,
}

impl Collector {
    /// Never block on a grandchild that keeps the pipe open.
    pub fn finish(self, grace: Duration, program: &str, pipe: &str) -> String {
        if let Err(err) = self.closed.recv_timeout(grace) {
            log::warn!("{program} exited but its {pipe} was not closed ({err}); output truncated");
        }

        let bytes =
            std::mem::take(&mut *self.buffer.lock().unwrap_or_else(PoisonError::into_inner));

        String::from_utf8(bytes)
            .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
    }
}

pub fn read_pipe(pipe: Option<impl Read + Send + 'static>) -> Collector {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let (sender, closed) = mpsc::channel();
    let shared = Arc::clone(&buffer);

    std::thread::spawn(move || {
        let mut chunk = vec![0_u8; CHUNK_BYTES];

        if let Some(mut pipe) = pipe {
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(count) => shared
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend(chunk.iter().take(count)),
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(err) => {
                        log::warn!("reading command output failed: {err}");
                        break;
                    }
                }
            }
        }

        if sender.send(()).is_err() {
            log::debug!("output reader finished after the collector stopped waiting");
        }
    });

    Collector { buffer, closed }
}

#[cfg(test)]
mod tests;
