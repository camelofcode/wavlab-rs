# wavlab

`wavlab` is an early Rust port of WavLab for reading, inspecting, creating,
and saving PCM WAV files.

The crate currently supports:

- Mono and stereo PCM WAV files
- 8-, 16-, 24-, and 32-bit integer samples
- Full sample loading or metadata-only loading
- Broadcast Wave (`bext`) chunks found before or after sample data
- Normalized internal sample values in the range `-1.0..=1.0`

## Adding the library

```toml
[dependencies]
wavlab = { path = "../wavlab-rs" }
```

```rust
use wavlab::{WavError, WavFile};
```

## Command-line interface

The package also includes a thin command-line interface. To inspect a WAV
file's metadata without loading its sample data, run:

```console
cargo run -- metadata input.wav
```

After installing the executable with `cargo install --path .`, the equivalent
command is:

```console
wavlab metadata input.wav
wavlab metadata --full input.wav
```

By default, `metadata` reads only the WAV structure. The `--full`/`-f` flag
also loads and scans the samples, adding peak and RMS levels in dBFS.

Audio can be normalized or amplified with the processing commands:

```console
wavlab normalize input.wav [output.wav]
wavlab normalize-db input.wav <target-db> [output.wav]
wavlab gain input.wav <linear-gain> [output.wav]
wavlab gain-db input.wav <gain-db> [output.wav]
wavlab reverse input.wav [output.wav]
wavlab flip-phase input.wav [output.wav]
wavlab swap-channels input.wav [output.wav]
wavlab split-stereo input.wav [left.wav right.wav]
wavlab split-mid-side input.wav [mid.wav side.wav]
wavlab split input.wav <milliseconds>...
wavlab left-right-to-stereo left.wav right.wav output.wav
wavlab mid-side-to-stereo mid.wav side.wav output.wav
wavlab mix output.wav <input.wav>...
wavlab join output.wav <input.wav>...
```

For the single-output processing commands, omitting the output path updates
the input file in place. Gain is not clamped while processing, so amplification
can cause clipping when the result is saved as integer PCM.

When both `split-stereo` output paths are omitted, they default to the input
name with `_l` and `_r` suffixes. For example, `audio.wav` produces
`audio_l.wav` and `audio_r.wav` in the same directory.

Likewise, omitted `split-mid-side` paths default to `_mid` and `_side`
suffixes. The `split` command writes numbered outputs such as `audio_1.wav`,
`audio_2.wav`, and `audio_3.wav`.

The `mix` and `join` commands require at least two inputs with matching sample
rates. Mixing starts every input at the first frame, promotes mono to stereo
when needed, pads shorter inputs with silence, and normalizes the result.
Joining requires a common channel count and concatenates the inputs in order.

## Creating an empty WAV file

`WavFile::new` accepts the number of channels, sample rate, and bits per
sample. It returns an error if the settings are unsupported.

```rust
use wavlab::{WavError, WavFile};

fn main() -> Result<(), WavError> {
    let wav_file = WavFile::new(2, 48_000, 24)?;
    wav_file.save("empty.wav")?;
    Ok(())
}
```

The supported channel counts are `1` and `2`. The supported bit depths are
`8`, `16`, `24`, and `32`.

## Opening a WAV file

`WavFile::open` reads the file metadata and loads all sample data:

```rust
use wavlab::{WavError, WavFile};

fn main() -> Result<(), WavError> {
    let wav_file = WavFile::open("input.wav")?;
    println!("{wav_file}");
    Ok(())
}
```

## Loading metadata only

`WavFile::open_metadata` reads the WAV structure without loading its sample
data. This is useful when inspecting large files:

```rust
use wavlab::{WavError, WavFile};

fn main() -> Result<(), WavError> {
    let wav_file = WavFile::open_metadata("input.wav")?;
    println!("{wav_file}");
    Ok(())
}
```

A metadata-only `WavFile` cannot be saved because it does not contain the
original samples. Calling `save` on it returns `WavError::SamplesNotLoaded`.

## Saving a WAV file

Files opened with `WavFile::open`, and files created with `WavFile::new`, can
be written with `save`:

```rust
use wavlab::{WavError, WavFile};

fn main() -> Result<(), WavError> {
    let wav_file = WavFile::open("input.wav")?;
    wav_file.save("output.wav")?;
    Ok(())
}
```

## Errors

All constructors and file operations return `Result<_, WavError>`. `WavError`
implements `std::error::Error`, so it can also be propagated through broader
application error types.

Errors include filesystem failures, malformed WAV structures, unsupported PCM
settings, incomplete multichannel sample frames, and WAV size limits.

## Current limitations

- Only integer PCM format WAV files are supported.
- Only mono and stereo files are supported.
- Unknown chunks are skipped while reading and are not reproduced when saving.
- A saved file is intended to be semantically equivalent, not byte-for-byte
  identical to its input.

