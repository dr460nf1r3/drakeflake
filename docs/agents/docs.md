# User documentation

## Layout and build

- `docs/src/SUMMARY.md` is the table of contents. A new page needs an entry, and
  its title must match.
- `scripts/build-docs.sh` builds into `docs/book`. `docs.yml` publishes every
  push to `main` and builds every pull request.
- Screenshots: `SELFTEST_DESKTOP_THEME=1 scripts/selftest.sh <suite>`, then copy
  from the run's `shots/`. Retina shots are 3200 x 1800. Scale to 1600 x 900
  with `sips -z 900 1600`.
- The app shows these pages in **Documentation**. The package copies `docs/src`
  at build time (`DRAKEFLAKE_DOCS_DIR`).
- The app renders Markdown text and tables, fenced code, `images/` paths, `<img>`
  lines and `> **Warning:**` or `> **Note:**` quotes. Use nothing else.
- A section's **Help** button opens a heading listed in `HELP_TOPICS`
  (`src/qml/main/helpers.js`). Rename such a heading together with that list.

## Rules

- No semicolons in prose, list items, table cells or alt text.
- Every bold UI label matches a `qsTr` string exactly.
- Every behaviour claim is backed by the code.
- Keep anchors that other pages link to. Grep for `page.md#anchor` first.
- Wrap prose at 80 columns.
- It is a desktop app. Never write "refresh the page" or "contact support".

## Page shapes

- A page covers every control of its area, matches the code today, and is
  written for a task.
- Anything that needs root, changes the system or deletes data gets a warning
  that says how to undo it.
- Use real option names, values and search terms. Use one word per thing.
- End each page with `## Related`, and two to four next pages.
- Voice: plain and second person. "Click **Apply**", not "Execute the apply
  operation".

## Troubleshooting and FAQ

Troubleshooting entries use `## <Problem in the user's words>`, then **Symptoms**,
**Cause**, **Solution** and, only when real, **Prevention**. FAQ entries use
`## Question?` with the answer first.
