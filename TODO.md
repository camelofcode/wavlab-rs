# C++ command porting checklist

This checklist tracks the commands registered by the C++ implementation in
`WavLab/Commands.cpp`. A checked item means equivalent functionality is
available in the Rust CLI, even if the Rust command currently has a different
name.

## Tools

- [x] `stat` (Rust: `metadata`)
- [x] `normalise` (Rust: `normalize`)
- [x] `reverse`
- [x] `flip-phase`
- [x] `swap-channels`
- [x] `split-stereo`
- [x] `left-right-to-stereo`
- [x] `split-mid-side`
- [x] `mid-side-to-stereo`
- [x] `get-peak-db` (Rust: `metadata --full`)
- [x] `get-rms-db` (Rust: `metadata --full`)
- [x] `gain`
- [x] `gain-db`
- [x] `mix`
- [x] `split`
- [x] `join`

## EQ

- [x] `eq-peak`
- [x] `eq-lowshelf`
- [x] `eq-highshelf`
- [x] `eq-allpass`
- [x] `eq-bandpass`
- [x] `eq-lowpass`
- [x] `eq-highpass`

## Effects

- [x] `saturate`
- [x] `digital-delay-mono`
- [x] `digital-delay-stereo`
- [x] `analog-delay-mono`
- [x] `analog-delay-stereo`
- [x] `tape-delay-mono`
- [x] `tape-delay-stereo`
- [x] `reverb`

## Generators

- [x] `sine-wave`
- [x] `square-wave`
- [x] `square-wave-additive`
- [x] `triangle-wave`
- [x] `triangle-wave-additive`
- [x] `sawtooth-wave`
- [x] `sawtooth-wave-additive`
- [x] `white-noise`
- [x] `pink-noise`

The Rust CLI also provides `normalize-db`, which has no separately registered
C++ command in `Commands.cpp`.
