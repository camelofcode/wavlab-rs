use std::io::{self, Read, Write};

const I24_MIN: i32 = -8_388_608;
const I24_MAX: i32 = 8_388_607;

pub(super) fn read_bytes<const N: usize>(reader: &mut impl Read) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn read_u8(reader: &mut impl Read) -> io::Result<u8> {
    Ok(read_bytes::<1>(reader)?[0])
}

pub(super) fn read_u16_le(reader: &mut impl Read) -> io::Result<u16> {
    Ok(u16::from_le_bytes(read_bytes(reader)?))
}

pub(super) fn read_i16_le(reader: &mut impl Read) -> io::Result<i16> {
    Ok(i16::from_le_bytes(read_bytes(reader)?))
}

pub(super) fn read_u32_le(reader: &mut impl Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_bytes(reader)?))
}

pub(super) fn read_i24_le(reader: &mut impl Read) -> io::Result<i32> {
    let bytes = read_bytes::<3>(reader)?;
    let sign_extension = if bytes[2] & 0x80 == 0 { 0x00 } else { 0xff };

    Ok(i32::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        sign_extension,
    ]))
}

pub(super) fn read_i32_le(reader: &mut impl Read) -> io::Result<i32> {
    Ok(i32::from_le_bytes(read_bytes(reader)?))
}

pub(super) fn write_bytes(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    writer.write_all(bytes)
}

pub(super) fn write_u8(writer: &mut impl Write, value: u8) -> io::Result<()> {
    writer.write_all(&[value])
}

pub(super) fn write_u16_le(writer: &mut impl Write, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

pub(super) fn write_i16_le(writer: &mut impl Write, value: i16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

pub(super) fn write_u32_le(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

pub(super) fn write_i24_le(writer: &mut impl Write, value: i32) -> io::Result<()> {
    if !(I24_MIN..=I24_MAX).contains(&value) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{value} does not fit in a signed 24-bit integer"),
        ));
    }

    writer.write_all(&value.to_le_bytes()[..3])
}

pub(super) fn write_i32_le(writer: &mut impl Write, value: i32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        read_bytes, read_i16_le, read_i24_le, read_i32_le, read_u8, read_u16_le, read_u32_le,
        write_bytes, write_i16_le, write_i24_le, write_i32_le, write_u8, write_u16_le,
        write_u32_le,
    };

    #[test]
    fn reads_little_endian_values() {
        let bytes = [
            0xab, 0x34, 0x12, 0xfe, 0xff, 0x78, 0x56, 0x34, 0x12, 0x88, 0xa9, 0xcb, 0xed,
        ];
        let mut reader = Cursor::new(bytes);

        assert_eq!(read_u8(&mut reader).unwrap(), 0xab);
        assert_eq!(read_u16_le(&mut reader).unwrap(), 0x1234);
        assert_eq!(read_i16_le(&mut reader).unwrap(), -2);
        assert_eq!(read_u32_le(&mut reader).unwrap(), 0x1234_5678);
        assert_eq!(read_i32_le(&mut reader).unwrap(), -0x1234_5678);
    }

    #[test]
    fn reads_signed_24_bit_boundaries() {
        let cases = [
            ([0x00, 0x00, 0x80], -8_388_608),
            ([0xff, 0xff, 0xff], -1),
            ([0x00, 0x00, 0x00], 0),
            ([0x01, 0x00, 0x00], 1),
            ([0xff, 0xff, 0x7f], 8_388_607),
        ];

        for (bytes, expected) in cases {
            assert_eq!(read_i24_le(&mut Cursor::new(bytes)).unwrap(), expected);
        }
    }

    #[test]
    fn writes_little_endian_values() {
        let mut bytes = Vec::new();

        write_u8(&mut bytes, 0xab).unwrap();
        write_u16_le(&mut bytes, 0x1234).unwrap();
        write_i16_le(&mut bytes, -2).unwrap();
        write_u32_le(&mut bytes, 0x1234_5678).unwrap();
        write_i32_le(&mut bytes, -0x1234_5678).unwrap();

        assert_eq!(
            bytes,
            [
                0xab, 0x34, 0x12, 0xfe, 0xff, 0x78, 0x56, 0x34, 0x12, 0x88, 0xa9, 0xcb, 0xed,
            ]
        );
    }

    #[test]
    fn writes_signed_24_bit_boundaries() {
        let cases = [
            (-8_388_608, [0x00, 0x00, 0x80]),
            (-1, [0xff, 0xff, 0xff]),
            (0, [0x00, 0x00, 0x00]),
            (1, [0x01, 0x00, 0x00]),
            (8_388_607, [0xff, 0xff, 0x7f]),
        ];

        for (value, expected) in cases {
            let mut bytes = Vec::new();
            write_i24_le(&mut bytes, value).unwrap();
            assert_eq!(bytes, expected);
        }
    }

    #[test]
    fn rejects_values_outside_the_signed_24_bit_range() {
        assert!(write_i24_le(&mut Vec::new(), -8_388_609).is_err());
        assert!(write_i24_le(&mut Vec::new(), 8_388_608).is_err());
    }

    #[test]
    fn reads_and_writes_arbitrary_bytes() {
        let mut bytes = Vec::new();
        write_bytes(&mut bytes, b"RIFF").unwrap();

        assert_eq!(read_bytes::<4>(&mut Cursor::new(bytes)).unwrap(), *b"RIFF");
    }

    #[test]
    fn reports_unexpected_end_of_input() {
        assert!(read_bytes::<4>(&mut Cursor::new([0x01, 0x02])).is_err());
    }
}
