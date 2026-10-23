#!/bin/sh
# Root never writes into a user-writable folder. The output file lives in a folder root creates in
# /private/var/run, and the exit status comes back as the last line `do shell script` returns.
set -eu
PATH=/usr/bin:/bin:/usr/sbin:/sbin

if [ "$#" -eq 0 ]; then
  echo "usage: drakeflake-elevate command [argument ...]" >&2
  exit 2
fi

quote() {
  printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

command=""
for argument in "$@"; do
  command="$command $(quote "$argument")"
done

# The folder name cannot be guessed ahead, and root creates it with `mkdir`,
# which fails on an existing path instead of following a planted link.
token=$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')
secure="/private/var/run/drakeflake-elevate.$token"
output="$secure/output"

work=$(mktemp -d -t drakeflake-elevate)
follower=""
trap 'if [ -n "$follower" ]; then kill "$follower" 2>/dev/null || true; wait "$follower" 2>/dev/null || true; fi; rm -rf "$work"' EXIT

if [ -t 0 ]; then
  : >"$work/input"
else
  cat >"$work/input"
fi

prompt="DrakeFlake wants to run as administrator:
$(printf '%s' "$*" | cut -c 1-300)"

# SUDO_UID tells the root helper whose files it may read (as with sudo).
# The script always ends with the status as its last output and exits 0, so osascript only fails
# when the dialog does. The pause lets the follower read the tail before the folder goes.
script="SUDO_UID=$(id -u) SUDO_USER=$(id -un); export SUDO_UID SUDO_USER;"
script="$script umask 022; status=1;"
script="$script if mkdir $(quote "$secure"); then"
script="$script ($command) <$(quote "$work/input") >$(quote "$output") 2>&1; status=\$?;"
script="$script sleep 1; rm -rf $(quote "$secure"); fi;"
script="$script echo \$status"

osascript - "$script" "$prompt" >"$work/result" 2>"$work/error" <<'APPLESCRIPT' &
on run argv
  do shell script (item 1 of argv) with prompt (item 2 of argv) with administrator privileges
end run
APPLESCRIPT
dialog=$!

while [ ! -e "$output" ] && kill -0 "$dialog" 2>/dev/null; do
  sleep 0.1
done

if [ -e "$output" ]; then
  tail -f "$output" &
  follower=$!
fi

wait "$dialog" || true

sleep 0.3

status=$(tr -dc '0-9' <"$work/result")
if [ -n "$status" ]; then
  exit "$status"
fi

if grep -q -- "-128" "$work/error"; then
  echo "Cancelled in the administrator dialog." >&2
  # pkexec ends a dismissed dialog with 127, which the app reads the same way.
  exit 127
fi

cat "$work/error" >&2
exit 1
