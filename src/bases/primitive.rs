#[inline]
fn extend_sign(val: u64, nbytes: usize) -> i64 {
    let shift = (8 - nbytes) * 8;
    (val << shift) as i64 >> shift
}
#[inline]
fn unextend_sign(val: i64, nbytes: usize) -> u64 {
    let shift = (8 - nbytes) * 8;
    (val << shift) as u64 >> shift
}

#[inline]
pub fn write_u8(val: u8) -> [u8; 1] {
    val.to_le_bytes()
}

#[inline]
pub fn write_u16(val: u16) -> [u8; 2] {
    val.to_le_bytes()
}

#[inline]
pub fn write_u32(val: u32) -> [u8; 4] {
    val.to_le_bytes()
}

#[inline]
pub fn write_u64(val: u64) -> [u8; 8] {
    val.to_le_bytes()
}

#[inline]
pub fn write_from_u64(val: u64, size: usize) -> [u8; 8] {
    debug_assert!(size <= 8);
    let mut buf = val.to_le_bytes();
    buf[size..].fill(0);
    buf
}

#[inline]
pub fn write_from_i64(val: i64, size: usize) -> [u8; 8] {
    debug_assert!(size <= 8);
    write_from_u64(unextend_sign(val, size), size)
}

#[inline]
pub fn read_u8(buf: &[u8]) -> u8 {
    debug_assert!(!buf.is_empty());
    u8::from_le_bytes((buf).try_into().unwrap())
}

#[inline]
pub fn read_u16(buf: &[u8]) -> u16 {
    debug_assert!(2 <= buf.len());
    u16::from_le_bytes((buf).try_into().unwrap())
}

#[inline]
pub fn read_u32(buf: &[u8]) -> u32 {
    debug_assert!(4 <= buf.len());
    u32::from_le_bytes((buf).try_into().unwrap())
}

#[inline]
pub fn read_u64(buf: &[u8]) -> u64 {
    debug_assert!(8 <= buf.len());
    u64::from_le_bytes((buf).try_into().unwrap())
}

#[inline]
pub fn read_to_u64(size: usize, buf: &[u8]) -> u64 {
    debug_assert!(size <= 8);
    let mut out = [0; 8];
    out[..size].copy_from_slice(&buf[..size]);
    u64::from_le_bytes(out)
}

#[inline]
pub fn read_i8(buf: &[u8]) -> i8 {
    debug_assert!(!buf.is_empty());
    i8::from_le_bytes(buf[..1].try_into().unwrap())
}

#[inline]
pub fn read_i16(buf: &[u8]) -> i16 {
    debug_assert!(2 <= buf.len());
    i16::from_le_bytes(buf.try_into().unwrap())
}

#[inline]
pub fn read_i32(buf: &[u8]) -> i32 {
    debug_assert!(4 <= buf.len());
    i32::from_le_bytes(buf.try_into().unwrap())
}

#[inline]
pub fn read_i64(buf: &[u8]) -> i64 {
    debug_assert!(8 <= buf.len());
    i64::from_le_bytes(buf.try_into().unwrap())
}

#[inline]
pub fn read_to_i64(size: usize, buf: &[u8]) -> i64 {
    extend_sign(read_to_u64(size, buf), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustest::test(params:(&'static [u8], u8)=[
        ([0x01].as_slice(), 1_u8),
        ([0xff].as_slice(), 255_u8),
        ([0x80].as_slice(), 128_u8),
    ])]
    fn read_u8_tests(Param((input, expected)): Param) {
        assert_eq!(read_u8(input), expected)
    }

    #[rustest::test(params:(&'static [u8], u16)=[
        ([0x01, 0x00].as_slice(), 1_u16),
        ([0x00, 0x01].as_slice(), 0x0100_u16),
        ([0xff, 0x00].as_slice(), 255_u16),
        ([0x80, 0x00].as_slice(), 128_u16),
        ([0x00, 0x80].as_slice(), 0x8000_u16),
        ([0xff, 0xff].as_slice(), 0xffff_u16)
    ])]
    fn read_u16_tests(Param((input, expected)): Param) {
        assert_eq!(read_u16(input), expected)
    }

    #[rustest::test(params:(&'static [u8], u32)=[
        ([0x00, 0x00, 0x00, 0x00].as_slice(), 0_u32),
        ([0x01, 0x00, 0x00, 0x00].as_slice(), 1_u32),
        ([0xff, 0x00, 0x00, 0x00].as_slice(), 255_u32),
        ([0x80, 0x00, 0x00, 0x00].as_slice(), 128_u32),
        ([0x00, 0x80, 0x00, 0x00].as_slice(), 0x8000_u32),
        ([0x00, 0x00, 0x80, 0x00].as_slice(), 0x800000_u32),
        ([0x00, 0x00, 0x00, 0x80].as_slice(), 0x80000000_u32),
        ([0x78, 0x56, 0x34, 0x12].as_slice(), 0x12345678_u32),
        ([0xff, 0xff, 0xff, 0xff].as_slice(), 0xffffffff_u32)
    ])]
    fn read_u32_tests(Param((input, expected)): Param) {
        assert_eq!(read_u32(input), expected)
    }

    #[rustest::test(params:(&'static [u8], u64)=[
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), 0_u64),
        ([0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), 1_u64),
        ([0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), 255_u64),
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff].as_slice(), 0xff00000000000000_u64),
        ([0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01].as_slice(), 0x0123456789abcdef_u64)
    ])]
    fn read_u64_tests(Param((input, expected)): Param) {
        assert_eq!(read_u64(input), expected)
    }

    #[rustest::test(params:(usize, u64)=[
        (1, 1_u64),
        (2, 0x23_01_u64),
        (3, 0x45_23_01_u64),
        (4, 0x67_45_23_01_u64),
        (5, 0x8967452301_u64),
        (6, 0xab8967452301_u64),
        (7, 0xcdab8967452301_u64),
        (8, 0xefcdab8967452301_u64)
    ])]
    fn read_to_u64_test(Param((size, expected)): Param) {
        assert_eq!(
            read_to_u64(size, &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            expected
        )
    }

    #[rustest::test(params:(u8, &'static [u8])=[
        (1_u8, [0x01].as_slice()),
        (255_u8, [0xff].as_slice()),
        (128_u8, [0x80].as_slice())
    ])]
    fn write_u8_tests(Param((input, expected)): Param) {
        assert_eq!(write_u8(input), expected)
    }

    #[rustest::test(params:(u16, &'static [u8])=[
        (1_u16, [0x01, 0x00].as_slice()),
        (0x0100_u16, [0x00, 0x01].as_slice()),
        (255_u16, [0xff, 0x00].as_slice()),
        (128_u16, [0x80, 0x00].as_slice()),
        (0x8000_u16, [0x00, 0x80].as_slice()),
        (0xffff_u16, [0xff, 0xff].as_slice())
    ])]
    fn write_u16_tests(Param((input, expected)): Param) {
        assert_eq!(write_u16(input), expected)
    }

    #[rustest::test(params:(u32, &'static [u8])=[
        (0_u32, [0x00, 0x00, 0x00, 0x00].as_slice()),
        (1_u32, [0x01, 0x00, 0x00, 0x00].as_slice()),
        (255_u32, [0xff, 0x00, 0x00, 0x00].as_slice()),
        (128_u32, [0x80, 0x00, 0x00, 0x00].as_slice()),
        (0x8000_u32, [0x00, 0x80, 0x00, 0x00].as_slice()),
        (0x800000_u32, [0x00, 0x00, 0x80, 0x00].as_slice()),
        (0x80000000_u32, [0x00, 0x00, 0x00, 0x80].as_slice()),
        (0x12345678_u32, [0x78, 0x56, 0x34, 0x12].as_slice()),
        (0xffffffff_u32, [0xff, 0xff, 0xff, 0xff].as_slice()),
    ])]
    fn write_u32_tests(Param((input, expected)): Param) {
        assert_eq!(write_u32(input), expected)
    }

    #[rustest::test(params:(u64, &'static [u8])=[
        (0_u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice()),
        (1_u64, [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice()),
        (255_u64, [0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice()),
        (0xff00000000000000_u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff].as_slice()),
        (0x0123456789abcdef_u64, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01].as_slice()),
    ])]
    fn write_u64_tests(Param((input, expected)): Param) {
        assert_eq!(write_u64(input), expected)
    }

    #[rustest::test(params:(usize, [u8;8])=[
        (1, [0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (2, [0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (3, [0xef, 0xcd, 0xab, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (4, [0xef, 0xcd, 0xab, 0x89, 0x00, 0x00, 0x00, 0x00]),
        (5, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x00, 0x00, 0x00]),
        (6, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x00, 0x00]),
        (7, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x00]),
        (8, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]),
    ])]
    fn write_from_u64_test(Param((size, expected)): Param) {
        assert_eq!(write_from_u64(0x0123456789abcdef_u64, size), expected)
    }

    #[rustest::test(params:(&'static[u8], i8)=[
        ([0x00].as_slice(), 0_i8),
        ([0xFF].as_slice(), -1_i8),
        ([0x01].as_slice(), 1_i8),
        ([0x80].as_slice(), i8::MIN),
        ([0x7F].as_slice(), i8::MAX),
        ([0xFE].as_slice(), -2_i8),
        ([0x7F, 0xff].as_slice(), 127_i8),
    ])]
    fn read_i8_tests(Param((input, expected)): Param) {
        assert_eq!(read_i8(input), expected)
    }

    #[rustest::test(params:(&'static[u8], i16)=[
        ([0x00, 0x00].as_slice(), 0_i16),
        ([0xFF, 0xFF].as_slice(), -1_i16),
        ([0x01, 0x00].as_slice(), 1_i16),
        ([0x80, 0xFF].as_slice(), i8::MIN as i16),
        ([0x7F, 0x00].as_slice(), i8::MAX as i16),
        ([0x00, 0x80].as_slice(), i16::MIN),
        ([0xFF, 0x7F].as_slice(), i16::MAX),
        ([0xDC, 0xFE].as_slice(), -292_i16),
    ])]
    fn read_i16_tests(Param((input, expected)): Param) {
        assert_eq!(read_i16(input), expected)
    }

    #[rustest::test(params:(&'static[u8], i32)=[
        ([0x00, 0x00, 0x00, 0x00].as_slice(), 0_i32),
        ([0xFF, 0xFF, 0xFF, 0xFF].as_slice(), -1_i32),
        ([0x01, 0x00, 0x00, 0x00].as_slice(), 1_i32),
        ([0x80, 0xFF, 0xFF, 0xFF].as_slice(), i8::MIN as i32),
        ([0x7F, 0x00, 0x00, 0x00].as_slice(), i8::MAX as i32),
        ([0x00, 0x80, 0xFF, 0xFF].as_slice(), i16::MIN as i32),
        ([0xFF, 0x7F, 0x00, 0x00].as_slice(), i16::MAX as i32),
        ([0x00, 0x00, 0x00, 0x80].as_slice(), i32::MIN),
        ([0xFF, 0xFF, 0xFF, 0x7F].as_slice(), i32::MAX),
        ([0x98, 0xBA, 0xDC, 0xFE].as_slice(), -19_088_744_i32),
    ])]
    fn read_i32_tests(Param((input, expected)): Param) {
        assert_eq!(read_i32(input), expected)
    }

    #[rustest::test(params:(&'static[u8], i64)=[
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), 0_i64),
        ([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].as_slice(), -1_i64),
        ([0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), 1_i64),
        ([0x80, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].as_slice(), i8::MIN as i64),
        ([0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), i8::MAX as i64),
        ([0x00, 0x80, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].as_slice(), i16::MIN as i64),
        ([0xFF, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_slice(), i16::MAX as i64),
        ([0x00, 0x00, 0x00, 0x80, 0xFF, 0xFF, 0xFF, 0xFF].as_slice(), i32::MIN as i64),
        ([0xFF, 0xFF, 0xFF, 0x7F, 0x00, 0x00, 0x00, 0x00].as_slice(), i32::MAX as i64),
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80].as_slice(), i64::MIN),
        ([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F].as_slice(), i64::MAX),
        ([0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE].as_slice(), -81_985_529_216_486_896_i64),
    ])]
    fn read_i64_tests(Param((input, expected)): Param) {
        assert_eq!(read_i64(input), expected)
    }

    #[rustest::test(params:(usize, i64)=[
        (1, 1_i64),
        (2, 0x23_01_i64),
        (3, 0x45_23_01_i64),
        (4, 0x67_45_23_01_i64),
        (5, 0xff_ff_ff_89_67_45_23_01_u64 as i64),
        (6, 0xff_ff_ab_89_67_45_23_01_u64 as i64),
        (7, 0xff_cd_ab_89_67_45_23_01_u64 as i64),
        (8, 0xef_cd_ab_89_67_45_23_01_u64 as i64),
    ])]
    fn read_to_i64_test(Param((size, expected)): Param) {
        assert_eq!(
            read_to_i64(size, &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            expected
        )
    }

    #[rustest::test(params:(usize, [u8; 8])=[
        (1, [0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (2, [0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (3, [0xef, 0xcd, 0xab, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (4, [0xef, 0xcd, 0xab, 0x89, 0x00, 0x00, 0x00, 0x00]),
        (5, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x00, 0x00, 0x00]),
        (6, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x00, 0x00]),
        (7, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x00]),
        (8, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]),
    ])]
    fn write_from_i64_positive_test(Param((size, expected)): Param) {
        assert_eq!(write_from_i64(0x0123456789abcdef_i64, size), expected)
    }

    #[rustest::test(params:(usize, [u8; 8])=[
        (1, [0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (2, [0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (3, [0xef, 0xcd, 0xab, 0x00, 0x00, 0x00, 0x00, 0x00]),
        (4, [0xef, 0xcd, 0xab, 0x89, 0x00, 0x00, 0x00, 0x00]),
        (5, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x00, 0x00, 0x00]),
        (6, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x00, 0x00]),
        (7, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x00]),
        (8, [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x81]),
    ])]
    fn write_from_i64_negative_test(Param((size, expected)): Param) {
        assert_eq!(
            write_from_i64(0x8123456789abcdef_u64 as i64, size),
            expected
        )
    }

    #[rustest::test(params:usize=[1, 2, 3, 4, 5, 6, 7, 8])]
    fn test_i64_in_and_out(Param(size): Param) {
        let ret = read_to_i64(size, &write_from_i64(2, size));
        assert_eq!(ret, 2);

        let ret = read_to_i64(size, &write_from_i64(-2, size));
        assert_eq!(ret, -2);

        let ret = read_to_i64(size, &write_from_i64(-128, size));
        assert_eq!(ret, -128);

        let ret = read_to_i64(size, &write_from_i64(127, size));
        assert_eq!(ret, 127);

        if size > 1 {
            let ret = read_to_i64(size, &write_from_i64(-10867, size));
            assert_eq!(ret, -10867);
        }
    }

    #[rustest::test(params:usize=[1, 2, 3, 4, 5, 6, 7, 8])]
    fn read_to_i64_minus_1(Param(size): Param) {
        assert_eq!(
            -1,
            read_to_i64(size, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
        );
    }
}
