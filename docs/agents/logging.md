# Logging

- Error: a user-requested operation failed.
- Warn: a fallback, a degraded path or a swallowed error.
- Info (the default): one line when a user action starts or finishes, when a
  setting changes, and when the target changes. Never one per item, step or
  evaluation.
- Debug: decisions, paths, counts and timings.
- Console failures go through `Hub::log_error` or `Hub::log_warn`, also from
  models, so each is logged once at the right level.
- `SystemRunner` logs commands. Yes or no checks use `Context::probe`, and runs
  expected to fail log at debug.
