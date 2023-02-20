use std::mem::{size_of, size_of_val};
use std::ptr::copy_nonoverlapping;

macro_rules! write_num_bytes {
    ($size:expr, $n:expr, $dst:expr) => {{
        debug_assert!($size <= $dst.len());
        let bytes = $n.to_be_bytes();
        unsafe {
            // N.B. https://github.com/rust-lang/rust/issues/22776
            copy_nonoverlapping(
                (&bytes)
                    .as_ptr()
                    .offset(size_of_val(&$n) as isize - ($size as isize)),
                $dst.as_mut_ptr(),
                $size,
            );
        }
    }};
}

macro_rules! read_num_bytes {
    ($size:expr, $buf:expr, $ty:ty) => {{
        debug_assert!($size <= $buf.len());
        let mut data: $ty = 0;
        unsafe {
            copy_nonoverlapping(
                $buf.as_ptr(),
                (&mut data as *mut $ty as *mut u8)
                    .offset(size_of::<$ty>() as isize - ($size as isize)),
                $size,
            )
        }
        <$ty>::from_be(data)
    }};
}

pub fn write_u8(val: u8, out: &mut [u8; 1]) {
    out[0] = val;
}

pub fn write_u16(val: u16, out: &mut [u8; 2]) {
    write_num_bytes!(2, val, out);
}

pub fn write_u32(val: u32, out: &mut [u8; 4]) {
    write_num_bytes!(4, val, out);
}

pub fn write_u64(val: u64, out: &mut [u8; 8]) {
    write_num_bytes!(8, val, out);
}

pub fn write_from_u64(val: u64, size: usize, out: &mut [u8]) {
    debug_assert!(size <= 8);
    write_num_bytes!(size, val, out);
}

pub fn write_from_i64(val: i64, size: usize, out: &mut [u8]) {
    debug_assert!(size <= 8);
    write_num_bytes!(size, val, out);
}

pub fn read_u8(buf: &[u8]) -> u8 {
    debug_assert!(!buf.is_empty());
    buf[0]
}

pub fn read_u16(buf: &[u8]) -> u16 {
    read_num_bytes!(2, buf, u16)
}

pub fn read_u32(buf: &[u8]) -> u32 {
    read_num_bytes!(4, buf, u32)
}

pub fn read_u64(buf: &[u8]) -> u64 {
    read_num_bytes!(8, buf, u64)
}

pub fn read_to_u64(size: usize, buf: &[u8]) -> u64 {
    debug_assert!(size <= 8);
    read_num_bytes!(size, buf, u64)
}

pub fn read_i8(buf: &[u8]) -> i8 {
    debug_assert!(!buf.is_empty());
    read_num_bytes!(1, buf, i8)
}

pub fn read_i16(buf: &[u8]) -> i16 {
    read_num_bytes!(2, buf, i16)
}

pub fn read_i32(buf: &[u8]) -> i32 {
    read_num_bytes!(4, buf, i32)
}

pub fn read_i64(buf: &[u8]) -> i64 {
    read_num_bytes!(8, buf, i64)
}

pub fn read_to_i64(size: usize, buf: &[u8]) -> i64 {
    debug_assert!(size > 0);
    debug_assert!(size <= 8);
    debug_assert!(size <= buf.len());
    let mut data = if buf[0].leading_zeros() == 0 {
        -1_i64
    } else {
        0_i64
    };
    unsafe {
        copy_nonoverlapping(
            buf.as_ptr(),
            (&mut data as *mut i64 as *mut u8).offset(8 - (size as isize)),
            size,
        )
    }
    i64::from_be(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(&[0x01] => 1_u8)]
    #[test_case(&[0xff] => 255_u8)]
    #[test_case(&[0x80] => 128_u8)]
    #[test_case(&[0x80, 0xff] => 128_u8)]
    fn read_u8_tests(input: &[u8]) -> u8 {
        read_u8(input)
    }

    #[test_case(&[0x00, 0x01] => 1_u16)]
    #[test_case(&[0x01, 0x00] => 0x0100_u16)]
    #[test_case(&[0x00, 0xff] => 255_u16)]
    #[test_case(&[0x00, 0x80] => 128_u16)]
    #[test_case(&[0x80, 0x00] => 0x8000_u16)]
    #[test_case(&[0xff, 0x0ff] => 0xffff_u16)]
    fn read_u16_tests(input: &[u8]) -> u16 {
        read_u16(input)
    }

    #[test_case(&[0x00, 0x00, 0x00, 0x00] => 0_u32)]
    #[test_case(&[0x00, 0x00, 0x00, 0x01] => 1_u32)]
    #[test_case(&[0x00, 0x00, 0x00, 0xff] => 255_u32)]
    #[test_case(&[0x00, 0x00, 0x00, 0x80] => 128_u32)]
    #[test_case(&[0x00, 0x00, 0x80, 0x00] => 0x8000_u32)]
    #[test_case(&[0x00, 0x80, 0x00, 0x00] => 0x800000_u32)]
    #[test_case(&[0x80, 0x00, 0x00, 0x00] => 0x80000000_u32)]
    #[test_case(&[0x12, 0x34, 0x56, 0x78] => 0x12345678_u32)]
    #[test_case(&[0xff, 0xff, 0xff, 0xff] => 0xffffffff_u32)]
    fn read_u32_tests(input: &[u8]) -> u32 {
        read_u32(input)
    }

    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] => 0_u64)]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01] => 1_u64)]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff] => 255_u64)]
    #[test_case(&[0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] => 0xff00000000000000_u64)]
    #[test_case(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef] => 0x0123456789abcdef_u64)]
    fn read_u64_test(input: &[u8]) -> u64 {
        read_u64(input)
    }

    #[test_case(1 => 1_u64)]
    #[test_case(2 => 0x01_23_u64)]
    #[test_case(3 => 0x01_23_45_u64)]
    #[test_case(4 => 0x01_23_45_67_u64)]
    #[test_case(5 => 0x0123456789_u64)]
    #[test_case(6 => 0x0123456789ab_u64)]
    #[test_case(7 => 0x0123456789abcd_u64)]
    #[test_case(8 => 0x0123456789abcdef_u64)]
    fn read_to_u64_test(size: usize) -> u64 {
        read_to_u64(size, &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef])
    }

    #[test_case(1_u8 => [0x01])]
    #[test_case(255_u8 => [0xff])]
    #[test_case(128_u8 => [0x80])]
    fn write_u8_tests(input: u8) -> [u8; 1] {
        let mut buf = [0; 1];
        write_u8(input, &mut buf);
        buf
    }

    #[test_case(1_u16 => [0x00, 0x01])]
    #[test_case(0x0100_u16 => [0x01, 0x00])]
    #[test_case(255_u16 => [0x00, 0xff])]
    #[test_case(128_u16 => [0x00, 0x80])]
    #[test_case(0x8000_u16 => [0x80, 0x00])]
    #[test_case(0xffff_u16 => [0xff, 0x0ff])]
    fn write_u16_tests(input: u16) -> [u8; 2] {
        let mut buf = [0; 2];
        write_u16(input, &mut buf);
        buf
    }

    #[test_case(0_u32 => [0x00, 0x00, 0x00, 0x00])]
    #[test_case(1_u32 => [0x00, 0x00, 0x00, 0x01])]
    #[test_case(255_u32 => [0x00, 0x00, 0x00, 0xff])]
    #[test_case(128_u32 => [0x00, 0x00, 0x00, 0x80])]
    #[test_case(0x8000_u32 => [0x00, 0x00, 0x80, 0x00])]
    #[test_case(0x800000_u32 => [0x00, 0x80, 0x00, 0x00])]
    #[test_case(0x80000000_u32 => [0x80, 0x00, 0x00, 0x00])]
    #[test_case(0x12345678_u32 => [0x12, 0x34, 0x56, 0x78])]
    #[test_case( 0xffffffff_u32 => [0xff, 0xff, 0xff, 0xff])]
    fn write_u32_tests(input: u32) -> [u8; 4] {
        let mut buf = [0; 4];
        write_u32(input, &mut buf);
        buf
    }

    #[test_case(0_u64 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(1_u64 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])]
    #[test_case(255_u64 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff])]
    #[test_case(0xff00000000000000_u64 => [0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(0x0123456789abcdef_u64 => [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef])]
    fn write_u64_test(input: u64) -> [u8; 8] {
        let mut buf = [0; 8];
        write_u64(input, &mut buf);
        buf
    }

    #[test_case(1 => [0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(2 => [0xcd, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(3 => [0xab, 0xcd, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(4 => [0x89, 0xab, 0xcd, 0xef, 0x00, 0x00, 0x00, 0x00])]
    #[test_case(5 => [0x67, 0x89, 0xab, 0xcd, 0xef, 0x00, 0x00, 0x00])]
    #[test_case(6 => [0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x00, 0x00])]
    #[test_case(7 => [0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x00])]
    #[test_case(8 => [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef])]
    fn write_from_u64_test(size: usize) -> [u8; 8] {
        let mut buf = [0; 8];
        write_from_u64(0x0123456789abcdef_u64, size, &mut buf);
        buf
    }

    #[test_case(&[0x00] => 0_i8; "0_i8")]
    #[test_case(&[0xFF] => -1_i8 ; "m1_i8")]
    #[test_case(&[0x01] => 1_i8; "p1_i8")]
    #[test_case(&[0x80] => i8::MIN; "i8min_i8")]
    #[test_case(&[0x7F] => i8::MAX; "i8max_i8")]
    #[test_case(&[0xFE] => -2_i8)]
    #[test_case(&[0x7F, 0xff] => 127_i8; "i8max_i8_garbage")]
    fn read_i8_tests(input: &[u8]) -> i8 {
        read_i8(input)
    }

    #[test_case(&[0x00, 0x00] => 0_i16; "0_i16")]
    #[test_case(&[0xFF, 0xFF] => -1_i16 ; "m1_i16")]
    #[test_case(&[0x00, 0x01] => 1_i16; "p1_i16")]
    #[test_case(&[0xFF, 0x80] => i8::MIN as i16; "i8min_i16")]
    #[test_case(&[0x00, 0x7F] => i8::MAX as i16; "i8max_i16")]
    #[test_case(&[0x80, 0x00] => i16::MIN; "i16min_i16")]
    #[test_case(&[0x7F, 0xFF] => i16::MAX; "i16max_i16")]
    #[test_case(&[0xFE, 0xDC] => -292_i16)]
    fn read_i16_tests(input: &[u8]) -> i16 {
        read_i16(input)
    }

    #[test_case(&[0x00, 0x00, 0x00, 0x00] => 0_i32; "0_i32")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0xFF] => -1_i32 ; "m1_i32")]
    #[test_case(&[0x00, 0x00, 0x00, 0x01] => 1_i32; "p1_i32")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0x80] => i8::MIN as i32; "i8min_i32")]
    #[test_case(&[0x00, 0x00, 0x00, 0x7F] => i8::MAX as i32; "i8max_i32")]
    #[test_case(&[0xFF, 0xFF, 0x80, 0x00] => i16::MIN as i32; "i16min_i32")]
    #[test_case(&[0x00, 0x00, 0x7F, 0xFF] => i16::MAX as i32; "i16max_i32")]
    #[test_case(&[0x80, 0x00, 0x00, 0x00] => i32::MIN; "i32min_i32")]
    #[test_case(&[0x7F, 0xFF, 0xFF, 0xFF] => i32::MAX; "i32max_i32")]
    #[test_case(&[0xFE, 0xDC, 0xBA, 0x98] => -19_088_744_i32)]
    fn read_i32_tests(input: &[u8]) -> i32 {
        read_i32(input)
    }

    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] => 0_i64; "0_i64")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] => -1_i64 ; "m1_i64")]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01] => 1_i64; "p1_i64")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x80] => i8::MIN as i64; "i8min_i64")]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F] => i8::MAX as i64; "i8max_i64")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x00] => i16::MIN as i64; "i16min_i64")]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0xFF] => i16::MAX as i64; "i16max_i64")]
    #[test_case(&[0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0x00, 0x00] => i32::MIN as i64; "i32min_i64")]
    #[test_case(&[0x00, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF] => i32::MAX as i64; "i32max_i64")]
    #[test_case(&[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] => i64::MIN; "i64min_i64")]
    #[test_case(&[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] => i64::MAX; "i64max_i64")]
    #[test_case(&[0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10] => -8_198_552_921_6486_896_i64)]
    fn read_i64_test(input: &[u8]) -> i64 {
        read_i64(input)
    }

    #[test_case(1 => 1_i64)]
    #[test_case(2 => 0x01_23_i64)]
    #[test_case(3 => 0x01_23_45_i64)]
    #[test_case(4 => 0x01_23_45_67_i64)]
    #[test_case(5 => 0x0123456789_i64)]
    #[test_case(6 => 0x0123456789ab_i64)]
    #[test_case(7 => 0x0123456789abcd_i64)]
    #[test_case(8 => 0x0123456789abcdef_i64)]
    fn read_to_i64_positive_test(size: usize) -> i64 {
        read_to_i64(size, &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef])
    }

    #[test_case(1)]
    #[test_case(2)]
    #[test_case(3)]
    #[test_case(4)]
    #[test_case(5)]
    #[test_case(6)]
    #[test_case(7)]
    #[test_case(8)]
    fn read_to_i64_minus_1(size: usize) {
        assert_eq!(
            -1,
            read_to_i64(size, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
        );
    }

    #[test_case(1 => -0x02_i64)]
    #[test_case(2 => -0x01_24_i64)]
    #[test_case(3 => -0x01_23_46_i64)]
    #[test_case(4 => -0x01_23_45_68_i64)]
    #[test_case(5 => -0x01_23_45_67_8A_i64)]
    #[test_case(6 => -0x01_23_45_67_89_AC_i64)]
    #[test_case(7 => -0x01_23_45_67_89_AB_CE_i64)]
    #[test_case(8 => -0x01_23_45_67_89_AB_CD_F0_i64)]
    fn read_to_i64_negative_test(size: usize) -> i64 {
        read_to_i64(size, &[0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10])
    }
}
