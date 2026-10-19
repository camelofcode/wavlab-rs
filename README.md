# wavlab

`wavlab` is a Rust library, command-line audio processor, and learning resource
for fundamental digital signal processing algorithms. It provides practical,
readable implementations of waveform and noise generation, filters, delays,
saturation, algorithmic reverb, level measurement, and other common audio
operations. Students can inspect how an algorithm works, apply it to a WAV
file, and hear the result.

The code favours clear, explicit implementations that can be studied and
adapted, while still exposing the processing through a reusable library and a
complete CLI application. Algorithm sources and further reading are collected
in the [references](#references).

It supports mono and stereo files containing 8-, 16-, 24-, or 32-bit integer
PCM. WAV samples can be loaded in full or inspected as metadata only. Broadcast
Wave (`bext`) chunks are preserved when files are saved.

## Build and install

Build the project locally:

```console
cargo build --release
./target/release/wavlab --help
```

Or install the executable from this checkout:

```console
cargo install --path .
```

## Command-line interface

For example, inspect a file without loading its samples, or include a full
peak and RMS scan:

```console
wavlab metadata input.wav
wavlab metadata --full input.wav
```

Run `wavlab --help` or see the [generated CLI reference](docs/cli.md) for every
command and argument.

## Library

Add the local checkout as a dependency:

```toml
[dependencies]
wavlab = { path = "../wavlab-rs" }
```

```rust
use wavlab::{WavError, WavFile};

fn main() -> Result<(), WavError> {
    let mut wav_file = WavFile::open("input.wav")?;
    wav_file.normalize_to_db(-1.0)?;
    wav_file.save("output.wav")?;
    Ok(())
}
```

Generate and open the complete library API documentation with:

```console
cargo doc --no-deps --open
```

## Documentation development

The CLI reference is generated from the same Clap definitions as `--help`:

```console
cargo run --example generate-cli-docs
```

The test suite checks that the committed reference is current.

## Current limitations

- Only integer PCM WAV files are supported.
- Only mono and stereo files are supported.
- Unknown chunks are skipped while reading and are not reproduced when saving.
- Saving reconstructs the WAV structure; output is not necessarily
  byte-for-byte identical to the input.

## References

The following technical publications and resources informed algorithm
implementations in this project. They are cited as background material and do
not represent bundled third-party source code.

- Robert Bristow-Johnson, “[Cookbook formulae for audio equalizer biquad filter
  coefficients](https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html),”
  adapted for the Web Audio specification by Raymond Toy and Doug Schepers.
- MusicDSP.org, “[C++ class implementation of RBJ
  Filters](https://www.musicdsp.org/en/latest/Filters/128-c-class-implementation-of-rbj-filters.html),”
  13 December 2002.
- Robin Whittle, “[DSP generation of Pink (1/f)
  Noise](https://www.firstpr.com.au/dsp/pink-noise/),” including Paul Kellet's
  pinking-filter coefficients, 18 October 1999.
- TAP-plugins, “[TAP Sigmoid
  Booster](https://tomscii.sig7.se/tap-plugins/ladspa/sigmoid.html),” describing
  the nonlinear transfer curve used for tape-style saturation.
- Manfred R. Schroeder, “[Natural Sounding Artificial
  Reverberation](https://aes.org/publications/elibrary/elibrary-page/?id=849),”
  *Journal of the Audio Engineering Society*, volume 10, issue 3, pages 219–223,
  July 1962. The reverb follows its parallel-comb, series-all-pass design.

## License

Copyright (C) 2026 Christopher Clayton.

This project is free software licensed under the
[GNU General Public License version 3 or later](LICENSE).

