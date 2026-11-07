# WavLab Rust repository guidance

Use the repository skills under `.agents/skills/` whenever their descriptions
match the task:

- `wavlab-rust-development` for library, CLI, test, Cargo, or documentation
  changes.
- `wavlab-rust-release` for version bumps, release preparation, tags, and
  GitHub Releases.

Keep reusable audio and WAV behaviour in the library. The binary should parse,
dispatch, format errors, and choose process status without duplicating DSP or
file-format logic.

Use British English in new prose. Preserve established Rust API and CLI
spellings such as `normalize` unless the task explicitly changes compatibility.
Generated `docs/cli.md` must come from the Clap definitions, not manual edits.

The `main` branch is protected. Make changes on a focused branch, open a pull
request, and require `Required Rust checks` to pass before merging.
