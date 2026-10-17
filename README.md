# wavlab

`wavlab` is a Rust library and command-line tool for reading, inspecting,
creating, processing, and saving PCM WAV files.

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

## License

Copyright (C) 2026 Christopher Clayton.

This project is free software licensed under the
[GNU General Public License version 3 or later](LICENSE).

