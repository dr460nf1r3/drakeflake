{ isLinux }:
[
  "--workspace"
  "--exclude"
  "drakeflake-lint"
  "--exclude"
  "drakeflake-functions"
  "--exclude"
  (if isLinux then "drakeflake-tray-macos" else "drakeflake-tray")
]
