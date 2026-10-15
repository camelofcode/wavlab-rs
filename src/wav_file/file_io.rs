use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use super::binary_io::{
    read_bytes, read_i16_le, read_i24_le, read_i32_le, read_u8, read_u16_le, read_u32_le,
    write_bytes, write_i16_le, write_i24_le, write_i32_le, write_u8, write_u16_le, write_u32_le,
};
use super::sample_conversion::{
    f64_to_pcm_i16, f64_to_pcm_i24, f64_to_pcm_i32, f64_to_pcm_u8, pcm_i16_to_f64, pcm_i24_to_f64,
    pcm_i32_to_f64, pcm_u8_to_f64,
};
use super::{
    BextChunk, BextChunkLocation, DataChunk, FileChunk, FormatChunk, PCM_FORMAT_CHUNK_SIZE,
    PcmLayout, WavError, WavFile, pcm_layout,
};

const RIFF_HEADER_SIZE: u64 = 12;
const CHUNK_HEADER_SIZE: u64 = 8;
const PCM_FORMAT_CODE: u16 = 1;

const BEXT_DESCRIPTION_SIZE: usize = 256;
const BEXT_ORIGINATOR_SIZE: usize = 32;
const BEXT_ORIGINATOR_REFERENCE_SIZE: usize = 32;
const BEXT_ORIGINATION_DATE_SIZE: usize = 10;
const BEXT_ORIGINATION_TIME_SIZE: usize = 8;
const BEXT_KNOWN_FIELDS_SIZE: u32 = 348;

struct SerializationInfo {
    layout: PcmLayout,
    data_size: u32,
    bext_size: Option<u32>,
    riff_size: u32,
}

impl WavFile {
    /// Opens a PCM WAV file and loads its samples into memory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WavError> {
        Self::open_internal(path.as_ref(), true)
    }

    /// Opens a PCM WAV file without loading its samples into memory.
    pub fn open_metadata(path: impl AsRef<Path>) -> Result<Self, WavError> {
        Self::open_internal(path.as_ref(), false)
    }

    /// Saves this PCM WAV file to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), WavError> {
        self.serialization_info()?;

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        self.write_to(&mut writer)?;
        writer.flush()?;
        Ok(())
    }

    fn open_internal(path: &Path, load_samples: bool) -> Result<Self, WavError> {
        let file = File::open(path)?;
        let length_in_bytes = file.metadata()?.len();
        let mut reader = BufReader::new(file);
        Self::read_from(&mut reader, length_in_bytes, load_samples)
    }

    fn read_from(
        reader: &mut (impl Read + Seek),
        length_in_bytes: u64,
        load_samples: bool,
    ) -> Result<Self, WavError> {
        if length_in_bytes < RIFF_HEADER_SIZE {
            return Err(invalid_file("file is too short to contain a RIFF header"));
        }

        let file_chunk = FileChunk {
            id: read_bytes(reader)?,
            size: read_u32_le(reader)?,
            format: read_bytes(reader)?,
        };

        if file_chunk.id != *b"RIFF" {
            return Err(invalid_file(format!(
                "expected RIFF identifier, found {}",
                four_cc(&file_chunk.id)
            )));
        }

        if file_chunk.format != *b"WAVE" {
            return Err(invalid_file(format!(
                "expected WAVE format, found {}",
                four_cc(&file_chunk.format)
            )));
        }

        let declared_length = u64::from(file_chunk.size) + 8;
        if declared_length != length_in_bytes {
            return Err(invalid_file(format!(
                "RIFF header declares {declared_length} bytes, but the file contains {length_in_bytes}"
            )));
        }

        let mut format_chunk = None;
        let mut bext_chunk = None;
        let mut data_chunk = None;

        while reader.stream_position()? < length_in_bytes {
            let chunk_header_start = reader.stream_position()?;
            if length_in_bytes - chunk_header_start < CHUNK_HEADER_SIZE {
                return Err(invalid_file("truncated chunk header"));
            }

            let id = read_bytes::<4>(reader)?;
            let size = read_u32_le(reader)?;
            let data_start = reader.stream_position()?;
            let data_end = data_start
                .checked_add(u64::from(size))
                .ok_or(WavError::SizeOverflow("chunk"))?;
            let padded_end = data_end
                .checked_add(u64::from(size % 2))
                .ok_or(WavError::SizeOverflow("chunk"))?;

            if padded_end > length_in_bytes {
                return Err(invalid_file(format!(
                    "{} chunk extends beyond the end of the file",
                    four_cc(&id)
                )));
            }

            match &id {
                b"fmt " => {
                    if format_chunk.is_some() {
                        return Err(invalid_file("multiple fmt chunks"));
                    }
                    if size < PCM_FORMAT_CHUNK_SIZE {
                        return Err(invalid_file(format!(
                            "fmt chunk is {size} bytes; PCM requires at least {PCM_FORMAT_CHUNK_SIZE}"
                        )));
                    }

                    format_chunk = Some(FormatChunk {
                        id,
                        size,
                        audio_format: read_u16_le(reader)?,
                        number_of_channels: read_u16_le(reader)?,
                        sample_rate: read_u32_le(reader)?,
                        byte_rate: read_u32_le(reader)?,
                        block_align: read_u16_le(reader)?,
                        bits_per_sample: read_u16_le(reader)?,
                    });
                }
                b"bext" => {
                    if bext_chunk.is_some() {
                        return Err(invalid_file("multiple BEXT chunks"));
                    }

                    let location = if data_chunk.is_some() {
                        BextChunkLocation::Footer
                    } else {
                        BextChunkLocation::Header
                    };
                    bext_chunk = Some(read_bext_chunk(reader, id, size, location)?);
                }
                b"data" => {
                    if data_chunk.is_some() {
                        return Err(invalid_file("multiple data chunks"));
                    }
                    data_chunk = Some(DataChunk {
                        id,
                        size,
                        start_offset: data_start,
                    });
                }
                _ => {}
            }

            reader.seek(SeekFrom::Start(padded_end))?;
        }

        let format_chunk = format_chunk.ok_or_else(|| invalid_file("missing fmt chunk"))?;
        let data_chunk = data_chunk.ok_or_else(|| invalid_file("missing data chunk"))?;

        if format_chunk.audio_format != PCM_FORMAT_CODE {
            return Err(invalid_file(format!(
                "audio format {} is not PCM",
                format_chunk.audio_format
            )));
        }

        let layout = pcm_layout(
            format_chunk.number_of_channels,
            format_chunk.sample_rate,
            format_chunk.bits_per_sample,
        )?;

        if format_chunk.block_align != layout.block_align {
            return Err(invalid_file(format!(
                "block alignment is {}, expected {}",
                format_chunk.block_align, layout.block_align
            )));
        }

        if format_chunk.byte_rate != layout.byte_rate {
            return Err(invalid_file(format!(
                "byte rate is {}, expected {}",
                format_chunk.byte_rate, layout.byte_rate
            )));
        }

        if data_chunk.size % u32::from(layout.block_align) != 0 {
            return Err(invalid_file(format!(
                "data chunk size {} does not contain complete {}-byte sample frames",
                data_chunk.size, layout.block_align
            )));
        }

        let sample_count_u64 = u64::from(data_chunk.size) / u64::from(layout.bytes_per_sample);
        let number_of_samples = usize::try_from(sample_count_u64)
            .map_err(|_| WavError::SizeOverflow("sample count"))?;

        let samples = if load_samples {
            reader.seek(SeekFrom::Start(data_chunk.start_offset))?;
            Some(read_samples(
                reader,
                format_chunk.bits_per_sample,
                number_of_samples,
            )?)
        } else {
            None
        };

        Ok(Self {
            length_in_bytes,
            file_chunk,
            format_chunk,
            bext_chunk,
            data_chunk,
            number_of_samples,
            samples,
        })
    }

    fn write_to(&self, writer: &mut impl Write) -> Result<(), WavError> {
        let info = self.serialization_info()?;
        let samples = self.samples.as_ref().ok_or(WavError::SamplesNotLoaded)?;

        write_bytes(writer, b"RIFF")?;
        write_u32_le(writer, info.riff_size)?;
        write_bytes(writer, b"WAVE")?;

        write_bytes(writer, b"fmt ")?;
        write_u32_le(writer, PCM_FORMAT_CHUNK_SIZE)?;
        write_u16_le(writer, PCM_FORMAT_CODE)?;
        write_u16_le(writer, self.format_chunk.number_of_channels)?;
        write_u32_le(writer, self.format_chunk.sample_rate)?;
        write_u32_le(writer, info.layout.byte_rate)?;
        write_u16_le(writer, info.layout.block_align)?;
        write_u16_le(writer, self.format_chunk.bits_per_sample)?;

        if let Some(bext_chunk) = self
            .bext_chunk
            .as_ref()
            .filter(|chunk| chunk.location == BextChunkLocation::Header)
        {
            write_bext_chunk(writer, bext_chunk, info.bext_size.expect("BEXT size"))?;
        }

        write_bytes(writer, b"data")?;
        write_u32_le(writer, info.data_size)?;
        write_samples(writer, self.format_chunk.bits_per_sample, samples)?;
        write_padding(writer, info.data_size)?;

        if let Some(bext_chunk) = self
            .bext_chunk
            .as_ref()
            .filter(|chunk| chunk.location == BextChunkLocation::Footer)
        {
            write_bext_chunk(writer, bext_chunk, info.bext_size.expect("BEXT size"))?;
        }

        Ok(())
    }

    fn serialization_info(&self) -> Result<SerializationInfo, WavError> {
        if self.format_chunk.audio_format != PCM_FORMAT_CODE {
            return Err(invalid_file(format!(
                "audio format {} is not PCM",
                self.format_chunk.audio_format
            )));
        }

        let layout = pcm_layout(
            self.format_chunk.number_of_channels,
            self.format_chunk.sample_rate,
            self.format_chunk.bits_per_sample,
        )?;
        let samples = self.samples.as_ref().ok_or(WavError::SamplesNotLoaded)?;

        if samples.len() % usize::from(self.format_chunk.number_of_channels) != 0 {
            return Err(WavError::IncompleteSampleFrame {
                sample_count: samples.len(),
                number_of_channels: self.format_chunk.number_of_channels,
            });
        }

        let data_size = samples
            .len()
            .checked_mul(usize::from(layout.bytes_per_sample))
            .and_then(|size| u32::try_from(size).ok())
            .ok_or(WavError::SizeOverflow("sample data"))?;

        let bext_size = self
            .bext_chunk
            .as_ref()
            .map(serialized_bext_size)
            .transpose()?;

        let mut file_size = RIFF_HEADER_SIZE
            .checked_add(serialized_chunk_size(PCM_FORMAT_CHUNK_SIZE))
            .and_then(|size| size.checked_add(serialized_chunk_size(data_size)))
            .ok_or(WavError::SizeOverflow("file"))?;

        if let Some(bext_size) = bext_size {
            file_size = file_size
                .checked_add(serialized_chunk_size(bext_size))
                .ok_or(WavError::SizeOverflow("file"))?;
        }

        let riff_size =
            u32::try_from(file_size - 8).map_err(|_| WavError::SizeOverflow("RIFF data"))?;

        Ok(SerializationInfo {
            layout,
            data_size,
            bext_size,
            riff_size,
        })
    }
}

fn read_samples(
    reader: &mut impl Read,
    bits_per_sample: u16,
    sample_count: usize,
) -> Result<Vec<f64>, WavError> {
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| WavError::SizeOverflow("sample data"))?;

    for _ in 0..sample_count {
        let sample = match bits_per_sample {
            8 => pcm_u8_to_f64(read_u8(reader)?),
            16 => pcm_i16_to_f64(read_i16_le(reader)?),
            24 => pcm_i24_to_f64(read_i24_le(reader)?),
            32 => pcm_i32_to_f64(read_i32_le(reader)?),
            _ => return Err(WavError::UnsupportedBitsPerSample(bits_per_sample)),
        };
        samples.push(sample);
    }

    Ok(samples)
}

fn write_samples(
    writer: &mut impl Write,
    bits_per_sample: u16,
    samples: &[f64],
) -> Result<(), WavError> {
    for &sample in samples {
        match bits_per_sample {
            8 => write_u8(writer, f64_to_pcm_u8(sample))?,
            16 => write_i16_le(writer, f64_to_pcm_i16(sample))?,
            24 => write_i24_le(writer, f64_to_pcm_i24(sample))?,
            32 => write_i32_le(writer, f64_to_pcm_i32(sample))?,
            _ => return Err(WavError::UnsupportedBitsPerSample(bits_per_sample)),
        }
    }

    Ok(())
}

fn read_bext_chunk(
    reader: &mut impl Read,
    id: [u8; 4],
    size: u32,
    location: BextChunkLocation,
) -> Result<BextChunk, WavError> {
    if size < BEXT_KNOWN_FIELDS_SIZE {
        return Err(invalid_file(format!(
            "BEXT chunk is {size} bytes; at least {BEXT_KNOWN_FIELDS_SIZE} are required"
        )));
    }

    let description = read_fixed_string::<BEXT_DESCRIPTION_SIZE>(reader)?;
    let originator = read_fixed_string::<BEXT_ORIGINATOR_SIZE>(reader)?;
    let originator_reference = read_fixed_string::<BEXT_ORIGINATOR_REFERENCE_SIZE>(reader)?;
    let origination_date = read_fixed_string::<BEXT_ORIGINATION_DATE_SIZE>(reader)?;
    let origination_time = read_fixed_string::<BEXT_ORIGINATION_TIME_SIZE>(reader)?;
    let time_reference_low = read_u32_le(reader)?;
    let time_reference_high = read_u32_le(reader)?;
    let version = read_u16_le(reader)?;

    let extra_size = usize::try_from(size - BEXT_KNOWN_FIELDS_SIZE)
        .map_err(|_| WavError::SizeOverflow("BEXT chunk"))?;
    let mut extra_data = Vec::new();
    extra_data
        .try_reserve_exact(extra_size)
        .map_err(|_| WavError::SizeOverflow("BEXT chunk"))?;
    extra_data.resize(extra_size, 0);
    reader.read_exact(&mut extra_data)?;

    Ok(BextChunk {
        location,
        id,
        size,
        description,
        originator,
        originator_reference,
        origination_date,
        origination_time,
        time_reference_low,
        time_reference_high,
        version,
        extra_data,
    })
}

fn write_bext_chunk(
    writer: &mut impl Write,
    bext_chunk: &BextChunk,
    size: u32,
) -> Result<(), WavError> {
    write_bytes(writer, b"bext")?;
    write_u32_le(writer, size)?;
    write_fixed_string::<BEXT_DESCRIPTION_SIZE>(writer, "description", &bext_chunk.description)?;
    write_fixed_string::<BEXT_ORIGINATOR_SIZE>(writer, "originator", &bext_chunk.originator)?;
    write_fixed_string::<BEXT_ORIGINATOR_REFERENCE_SIZE>(
        writer,
        "originator reference",
        &bext_chunk.originator_reference,
    )?;
    write_fixed_string::<BEXT_ORIGINATION_DATE_SIZE>(
        writer,
        "origination date",
        &bext_chunk.origination_date,
    )?;
    write_fixed_string::<BEXT_ORIGINATION_TIME_SIZE>(
        writer,
        "origination time",
        &bext_chunk.origination_time,
    )?;
    write_u32_le(writer, bext_chunk.time_reference_low)?;
    write_u32_le(writer, bext_chunk.time_reference_high)?;
    write_u16_le(writer, bext_chunk.version)?;
    write_bytes(writer, &bext_chunk.extra_data)?;
    write_padding(writer, size)?;
    Ok(())
}

fn read_fixed_string<const N: usize>(reader: &mut impl Read) -> Result<String, WavError> {
    let bytes = read_bytes::<N>(reader)?;
    let end = bytes
        .iter()
        .rposition(|&byte| byte != 0)
        .map_or(0, |index| index + 1);
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn write_fixed_string<const N: usize>(
    writer: &mut impl Write,
    field: &'static str,
    value: &str,
) -> Result<(), WavError> {
    let bytes = value.as_bytes();
    if bytes.len() > N {
        return Err(WavError::TextFieldTooLong {
            field,
            maximum: N,
            actual: bytes.len(),
        });
    }

    write_bytes(writer, bytes)?;
    write_zeroes(writer, N - bytes.len())?;
    Ok(())
}

fn serialized_bext_size(bext_chunk: &BextChunk) -> Result<u32, WavError> {
    validate_text_length(
        "description",
        &bext_chunk.description,
        BEXT_DESCRIPTION_SIZE,
    )?;
    validate_text_length("originator", &bext_chunk.originator, BEXT_ORIGINATOR_SIZE)?;
    validate_text_length(
        "originator reference",
        &bext_chunk.originator_reference,
        BEXT_ORIGINATOR_REFERENCE_SIZE,
    )?;
    validate_text_length(
        "origination date",
        &bext_chunk.origination_date,
        BEXT_ORIGINATION_DATE_SIZE,
    )?;
    validate_text_length(
        "origination time",
        &bext_chunk.origination_time,
        BEXT_ORIGINATION_TIME_SIZE,
    )?;

    usize::try_from(BEXT_KNOWN_FIELDS_SIZE)
        .ok()
        .and_then(|size| size.checked_add(bext_chunk.extra_data.len()))
        .and_then(|size| u32::try_from(size).ok())
        .ok_or(WavError::SizeOverflow("BEXT chunk"))
}

fn validate_text_length(field: &'static str, value: &str, maximum: usize) -> Result<(), WavError> {
    let actual = value.len();
    if actual > maximum {
        Err(WavError::TextFieldTooLong {
            field,
            maximum,
            actual,
        })
    } else {
        Ok(())
    }
}

fn write_padding(writer: &mut impl Write, chunk_size: u32) -> Result<(), WavError> {
    if !chunk_size.is_multiple_of(2) {
        write_u8(writer, 0)?;
    }
    Ok(())
}

fn write_zeroes(writer: &mut impl Write, mut count: usize) -> Result<(), WavError> {
    const ZEROES: [u8; 256] = [0; 256];

    while count > 0 {
        let amount = count.min(ZEROES.len());
        write_bytes(writer, &ZEROES[..amount])?;
        count -= amount;
    }

    Ok(())
}

fn serialized_chunk_size(data_size: u32) -> u64 {
    CHUNK_HEADER_SIZE + u64::from(data_size) + u64::from(data_size % 2)
}

fn invalid_file(message: impl Into<String>) -> WavError {
    WavError::InvalidFile(message.into())
}

fn four_cc(bytes: &[u8; 4]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{BEXT_KNOWN_FIELDS_SIZE, WavError, WavFile};
    use crate::wav_file::{BextChunk, BextChunkLocation};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    const MONO_16_BIT_FIXTURE: [u8; 50] = [
        b'R', b'I', b'F', b'F', 42, 0, 0, 0, b'W', b'A', b'V', b'E', b'f', b'm', b't', b' ', 16, 0,
        0, 0, 1, 0, 1, 0, 0x40, 0x1f, 0, 0, 0x80, 0x3e, 0, 0, 2, 0, 16, 0, b'd', b'a', b't', b'a',
        6, 0, 0, 0, 0x00, 0x80, 0x00, 0x00, 0xff, 0x7f,
    ];

    #[test]
    fn reads_and_writes_a_known_pcm_fixture() {
        let loaded = WavFile::read_from(
            &mut Cursor::new(MONO_16_BIT_FIXTURE),
            MONO_16_BIT_FIXTURE.len() as u64,
            true,
        )
        .unwrap();

        assert_eq!(loaded.format_chunk.number_of_channels, 1);
        assert_eq!(loaded.format_chunk.sample_rate, 8_000);
        assert_eq!(loaded.format_chunk.bits_per_sample, 16);
        assert_eq!(loaded.samples, Some(vec![-1.0, 0.0, 1.0]));

        let mut encoded = Vec::new();
        loaded.write_to(&mut encoded).unwrap();
        assert_eq!(encoded, MONO_16_BIT_FIXTURE);
    }

    #[test]
    fn round_trips_all_supported_pcm_bit_depths() {
        for bits_per_sample in [8, 16, 24, 32] {
            let original_samples = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
            let mut wav_file = WavFile::new(1, 48_000, bits_per_sample).unwrap();
            wav_file.number_of_samples = original_samples.len();
            wav_file.samples = Some(original_samples.clone());

            let mut bytes = Vec::new();
            wav_file.write_to(&mut bytes).unwrap();
            let loaded = WavFile::read_from(
                &mut Cursor::new(&bytes),
                u64::try_from(bytes.len()).unwrap(),
                true,
            )
            .unwrap();

            assert_eq!(loaded.format_chunk.bits_per_sample, bits_per_sample);
            assert_eq!(loaded.number_of_samples, original_samples.len());
            assert_eq!(
                loaded.data_chunk.size,
                original_samples.len() as u32 * (bits_per_sample / 8) as u32
            );

            let tolerance = 1.0 / f64::from((1_u32 << (bits_per_sample - 1)) - 1);
            for (actual, expected) in loaded.samples.unwrap().iter().zip(original_samples) {
                assert!((actual - expected).abs() <= tolerance);
            }
        }
    }

    #[test]
    fn metadata_open_skips_samples_but_keeps_their_count() {
        let mut wav_file = WavFile::new(2, 48_000, 24).unwrap();
        wav_file.samples = Some(vec![-1.0, 0.0, 0.5, 1.0]);
        wav_file.number_of_samples = 4;

        let mut bytes = Vec::new();
        wav_file.write_to(&mut bytes).unwrap();
        let loaded = WavFile::read_from(
            &mut Cursor::new(&bytes),
            u64::try_from(bytes.len()).unwrap(),
            false,
        )
        .unwrap();

        assert_eq!(loaded.number_of_samples, 4);
        assert!(loaded.samples.is_none());
        assert!(matches!(
            loaded.write_to(&mut Vec::new()),
            Err(WavError::SamplesNotLoaded)
        ));
    }

    #[test]
    fn preserves_bext_header_and_footer_locations() {
        for location in [BextChunkLocation::Header, BextChunkLocation::Footer] {
            let mut wav_file = WavFile::new(1, 48_000, 16).unwrap();
            wav_file.bext_chunk = Some(BextChunk {
                location,
                id: *b"bext",
                size: 602,
                description: "Test recording".into(),
                originator: "wavlab".into(),
                originator_reference: "reference".into(),
                origination_date: "2026-10-16".into(),
                origination_time: "12:34:56".into(),
                time_reference_low: 123,
                time_reference_high: 456,
                version: 0,
                extra_data: vec![0; usize::try_from(602 - BEXT_KNOWN_FIELDS_SIZE).unwrap()],
            });

            let mut bytes = Vec::new();
            wav_file.write_to(&mut bytes).unwrap();
            let loaded = WavFile::read_from(
                &mut Cursor::new(&bytes),
                u64::try_from(bytes.len()).unwrap(),
                true,
            )
            .unwrap();
            let loaded_bext = loaded.bext_chunk.unwrap();

            assert_eq!(loaded_bext.location, location);
            assert_eq!(loaded_bext.description, "Test recording");
            assert_eq!(loaded_bext.size, 602);
        }
    }

    #[test]
    fn public_save_and_open_methods_round_trip_a_file() {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("wavlab-rs-{}-{sequence}.wav", std::process::id()));

        let mut wav_file = WavFile::new(1, 44_100, 24).unwrap();
        wav_file.samples = Some(vec![-1.0, 0.0, 1.0]);
        wav_file.number_of_samples = 3;
        wav_file.save(&path).unwrap();

        let loaded = WavFile::open(&path).unwrap();
        let metadata = WavFile::open_metadata(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(loaded.number_of_samples, 3);
        assert_eq!(loaded.samples.unwrap(), vec![-1.0, 0.0, 1.0]);
        assert_eq!(metadata.number_of_samples, 3);
        assert!(metadata.samples.is_none());
    }
}
