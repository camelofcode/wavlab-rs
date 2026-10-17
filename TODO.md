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

- [ ] `eq-peak`
- [ ] `eq-lowshelf`
- [ ] `eq-highshelf`
- [ ] `eq-allpass`
- [ ] `eq-bandpass`
- [ ] `eq-lowpass`
- [ ] `eq-highpass`

## Effects

- [ ] `saturate`
- [ ] `digital-delay-mono`
- [ ] `digital-delay-stereo`
- [ ] `analog-delay-mono`
- [ ] `analog-delay-stereo`
- [ ] `tape-delay-mono`
- [ ] `tape-delay-stereo`
- [ ] `reverb`

## Generators

- [ ] `sine-wave`
- [ ] `square-wave`
- [ ] `square-wave-additive`
- [ ] `triangle-wave`
- [ ] `triangle-wave-additive`
- [ ] `sawtooth-wave`
- [ ] `sawtooth-wave-additive`
- [ ] `white-noise`
- [ ] `pink-noise`

The Rust CLI also provides `normalize-db`, which has no separately registered
C++ command in `Commands.cpp`.
