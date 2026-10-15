# wavlab-rs

`wavlab-rs` is an early Rust port of WavLab for reading, inspecting, creating,
and saving PCM WAV files.

The crate currently supports:

- Mono and stereo PCM WAV files
- 8-, 16-, 24-, and 32-bit integer samples
- Full sample loading or metadata-only loading
- Broadcast Wave (`bext`) chunks found before or after sample data
- Normalized internal sample values in the range `-1.0..=1.0`

## Adding the library

The package name uses a hyphen, while Rust code imports it with an underscore:

```toml
[dependencies]
wavlab-rs = { path = "../wavlab-rs" }
```

```rust
use wavlab_rs::{WavError, WavFile};
```

## Creating an empty WAV file

`WavFile::new` accepts the number of channels, sample rate, and bits per
sample. It returns an error if the settings are unsupported.

```rust
use wavlab_rs::{WavError, WavFile};

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
use wavlab_rs::{WavError, WavFile};

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
use wavlab_rs::{WavError, WavFile};

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
use wavlab_rs::{WavError, WavFile};

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
- Sample access and mutation methods are not public yet.
- Unknown chunks are skipped while reading and are not reproduced when saving.
- A saved file is intended to be semantically equivalent, not byte-for-byte
  identical to its input.

