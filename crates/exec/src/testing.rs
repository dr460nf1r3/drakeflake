use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::cancel::CancelToken;
use crate::context::Context;
use crate::error::ExecError;
use crate::process::{Captured, Invocation};
use crate::runner::Runner;

type Reply = Result<Captured, ExecError>;
type Script = dyn Fn(&Call) -> Reply + Send + Sync;

#[derive(Debug, Clone)]
pub struct Call {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub input: Option<String>,
    pub cwd: Option<PathBuf>,
    pub timeout: Duration,
    pub cancel: Option<CancelToken>,
}

impl Call {
    fn of(invocation: &Invocation<'_>) -> Self {
        Self {
            program: invocation.program.clone(),
            args: invocation.args.clone(),
            input: invocation.input.map(str::to_owned),
            cwd: invocation.cwd.map(PathBuf::from),
            timeout: invocation.timeout,
            cancel: invocation.cancel.cloned(),
        }
    }

    #[must_use = "only reads the cancel flag"]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.as_ref().is_some_and(CancelToken::is_cancelled)
    }

    #[must_use = "only reads the arguments"]
    pub fn has_arg(&self, arg: impl AsRef<OsStr>) -> bool {
        self.args.iter().any(|candidate| candidate == arg.as_ref())
    }

    #[must_use = "only reads the arguments"]
    pub fn arg_after(&self, flag: impl AsRef<OsStr>) -> Option<&str> {
        let mut args = self.args.iter();

        args.find(|arg| *arg == flag.as_ref())?;
        args.next()?.to_str()
    }
}

pub struct ScriptedRunner {
    script: Box<Script>,
    calls: Mutex<Vec<Call>>,
}

impl ScriptedRunner {
    #[must_use = "the runner and its context are only built"]
    pub fn start(script: impl Fn(&Call) -> Reply + Send + Sync + 'static) -> (Arc<Self>, Context) {
        let runner = Arc::new(Self {
            script: Box::new(script),
            calls: Mutex::new(Vec::new()),
        });

        let handle = Arc::clone(&runner);
        let shared: Arc<dyn Runner> = handle;
        (runner, Context::new(shared))
    }

    #[must_use = "the runner and its context are only built"]
    pub fn answering(replies: Vec<Reply>) -> (Arc<Self>, Context) {
        let queue = Mutex::new(
            replies
                .into_iter()
                .collect::<std::collections::VecDeque<_>>(),
        );

        Self::start(move |call| {
            lock(&queue).pop_front().unwrap_or_else(|| {
                Err(ExecError::Failed(format!(
                    "no scripted reply left for {}",
                    call.program.to_string_lossy()
                )))
            })
        })
    }

    #[must_use = "only reads the recorded calls"]
    pub fn calls(&self) -> Vec<Call> {
        lock(&self.calls).clone()
    }
}

impl Runner for ScriptedRunner {
    fn run(&self, invocation: Invocation<'_>) -> Reply {
        let call = Call::of(&invocation);

        lock(&self.calls).push(call.clone());
        (self.script)(&call)
    }
}

#[must_use = "the reply is only built"]
pub fn success(stdout: &str) -> Captured {
    Captured {
        success: true,
        code: Some(0),
        stdout: stdout.to_owned(),
        stderr: String::new(),
    }
}

#[must_use = "the reply is only built"]
pub fn failure(stderr: &str) -> Captured {
    Captured {
        success: false,
        code: Some(1),
        stdout: String::new(),
        stderr: stderr.to_owned(),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
