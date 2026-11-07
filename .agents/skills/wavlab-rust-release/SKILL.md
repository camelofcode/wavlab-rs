---
name: wavlab-rust-release
description: Prepare and publish a WavLab Rust version, including Cargo version consistency, changelog, protected-branch CI, an immutable tag, and a GitHub Release.
---

# Release WavLab Rust

Use this skill when preparing, tagging, publishing, or auditing a Rust release.
Publishing tags and GitHub Releases changes external state: do it only when the
user has explicitly requested publication.

## Release invariants

- Use semantic versions and tags of the form `vX.Y.Z`.
- `Cargo.toml`, the root package entry in `Cargo.lock`, binary `--version`, the
  changelog heading, tag, and GitHub Release must agree.
- Tags are immutable release records. Never move, replace, or delete a
  published release tag without explicit user direction.
- Release only a commit on protected `main` whose `Required Rust checks`
  passed. Do not publish directly from an unmerged preparation branch.
- A GitHub Release does not imply publishing to crates.io. Publish the crate
  only when the user separately requests it and crates.io metadata is ready.

## Prepare the release

1. Confirm `main` is clean and synchronized; inspect existing tags/releases so
   the version is unused.
2. Create a focused `release/vX.Y.Z` branch.
3. Update the package version in `Cargo.toml`, then let Cargo update the root
   entry in `Cargo.lock`. Do not hand-edit dependency versions.
4. Keep an empty `Unreleased` section at the top of `CHANGELOG.md`; move the
   accumulated entries into `## [X.Y.Z] - YYYY-MM-DD`.
5. Regenerate `docs/cli.md` only if the Clap definitions changed.
6. Run the full gate from `wavlab-rust-development`, including rustdoc.
7. Commit, push, open a PR, and wait for `Required Rust checks`. Merge through
   the protected branch.

## Publish

1. Synchronize local `main` and verify the merge commit's required workflow is
   successful.
2. Recheck that the tag and GitHub Release do not already exist.
3. Create an annotated tag on the exact `main` commit:

   `git tag -a vX.Y.Z -m "WavLab Rust vX.Y.Z"`

4. Push that exact tag, then create the GitHub Release from it using the
   matching changelog section as notes.
5. Verify with `git ls-remote --tags`, `gh release view`, and `cargo metadata`.
   Confirm `cargo run -- --version` reports `X.Y.Z`.
6. Report the tagged commit and release URL. Do not silently run
   `cargo publish` or attach binary artefacts.
