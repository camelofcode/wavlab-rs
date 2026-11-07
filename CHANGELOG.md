# Changelog

All notable changes to the Rust implementation are documented here.

## [Unreleased]

## [0.1.0] - 2026-11-07

The first coordinated WavLab release provides a reusable Rust library and a
complete command-line audio processor.

### Added

- Integer PCM WAV reading and writing for mono and stereo 8-, 16-, 24-, and
  32-bit files, including metadata-only reads and BEXT metadata preservation.
- Typed library errors and failure-safe file replacement.
- Gain, normalisation, level measurement, frame transforms, stereo and
  mid/side conversion, splitting, mixing, and joining.
- Direct and additive waveform generation plus white and pink noise.
- Peaking, shelving, all-pass, band-pass, low-pass, and high-pass biquads.
- Tape saturation; digital, analogue-style, and tape-style delays; and
  Schroeder reverb.
- Chorus, vibrato, flanging, tremolo, automatic panning, phasing, and
  LFO-modulated low/high-pass filtering.
- A Clap command line with generated reference documentation and comprehensive
  library and process-level tests.
- Cross-platform formatting, lint, and test CI for Linux, Windows, and macOS.
