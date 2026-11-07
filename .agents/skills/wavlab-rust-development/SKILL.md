---
name: wavlab-rust-development
description: Maintain WavLab Rust library, CLI, Cargo metadata, documentation, and tests while preserving the crate's architecture and behavioural conventions.
---

# WavLab Rust development

Use this skill for implementation, refactoring, review, or documentation work
in the Rust repository. For tags and GitHub Releases, use
`wavlab-rust-release` instead.

## Preserve the architecture

- `src/lib.rs` defines the public crate surface. Re-export intentional public
  types there; keep implementation modules private unless consumers need them.
- WAV parsing/model logic belongs under `src/wav_file`; editing primitives in
  `src/tools`; generators in `src/generators`; filters in `src/eq`; effects in
  `src/fx`.
- `src/cli.rs` owns Clap types and generated reference documentation.
  `src/main.rs` owns process-level dispatch, file orchestration, output, and
  exit behaviour. Do not duplicate DSP algorithms in either CLI file.

## Library conventions

- Use Rust 2024, rustfmt defaults, snake_case items, and existing settings
  structs for multi-parameter effects.
- Public items require useful rustdoc because the crate enables
  `#![warn(missing_docs)]`. Document units, ranges, mutation, channel behaviour,
  and `# Errors` for fallible methods.
- Return the existing typed `WavError`, `ProcessingError`, `GeneratorError`,
  `DelayError`, or `EffectError` variants. Do not panic on user data or use
  stringly typed library errors.
- Validate all settings and checked sizes before replacing samples. Preserve
  the original object when an operation fails.
- Treat samples as interleaved channel values and frames as complete channel
  groups. Keep independent state per channel.
- Do not implicitly normalise effect output. PCM encoding owns final clipping.
- Use British English in prose. Preserve established public Rust spellings
  such as `normalize` and Clap command names unless compatibility is explicitly
  in scope.

## CLI and generated documentation

- Put argument semantics, validation metadata, help, aliases, and value enums
  in the Clap definitions. Keep dispatch branches thin and call library APIs.
- Preserve the deliberate omitted-output overwrite behaviour and paired-output
  requirements.
- After changing CLI definitions, regenerate `docs/cli.md` with:

  `cargo run --example generate-cli-docs`

- Never hand-edit generated command help. The test suite checks it exactly.

## Tests and completion

- Keep focused unit tests beside the module under `#[cfg(test)]`. Keep
  process/dispatch tests in the existing binary test module and use temporary
  files rather than repository output.
- Cover success, invalid input, unchanged state, empty/boundary input, and
  mono/stereo behaviour where meaningful. Test invariants for random noise,
  not exact random samples.
- Run the normal local gate:

  ```console
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features --locked -- -D warnings
  cargo test --all-targets --all-features --locked
  cargo doc --no-deps --all-features
  ```

- Let GitHub Actions provide Linux, Windows, and macOS coverage. Do not add
  generated build output under `target/`.
