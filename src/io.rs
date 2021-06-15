
use std::ptr::copy_nonoverlapping;
use std::convert::TryInto;
use std::mem::{size_of, size_of_val};

#[derive(Debug,PartialEq)]
pub struct SerialError {}

macro_rules! write_num_bytes {
    ($size:expr, $n:expr, $dst:expr) => ({
        assert!($size <= $dst.len());
        unsafe {
            // N.B. https://github.com/rust-lang/rust/issues/22776
            let bytes = $n.to_be_bytes();
            copy_nonoverlapping(
                (&bytes).as_ptr().offset(size_of_val(&$n) as isize-($size as isize)),
                $dst.as_mut_ptr(),
                $size);
        }
    });
}

macro_rules! read_num_bytes {
    ($size:expr, $buf:expr, $ty:ty) => ({
        assert!($size <= $buf.len());
        let mut data: $ty = 0;
        unsafe {
            copy_nonoverlapping(
                $buf.as_ptr(),
                (&mut data as *mut $ty as *mut u8).offset(size_of::<$ty>() as isize - ($size as isize)),
                $size
            )
        }
        <$ty>::from_be(data)
    })
}

pub fn write_u8(val: u8, out: &mut[u8;1])
{
    out[0] = val;
}

pub fn write_u16(val: u16, out: &mut[u8;2])
{
    write_num_bytes!(2, val, out);
}

pub fn write_u24(val: u32, out: &mut[u8;3])
{
    write_num_bytes!(3, val, out);
}

pub fn write_u32(val: u32, out: &mut[u8;4])
{
    write_num_bytes!(4, val, out);
}

pub fn write_u40(val: u64, out: &mut[u8;5])
{
    write_num_bytes!(5, val, out);
}

pub fn write_u48(val: u64, out: &mut[u8;6])
{
    write_num_bytes!(6, val, out);
}

pub fn write_u56(val: u64, out: &mut[u8;7])
{
    write_num_bytes!(7, val, out);
}

pub fn write_u64(val: u64, out: &mut[u8;8])
{
    write_num_bytes!(8, val, out);
}

pub fn write_from_u64(val: u64, size:usize, out:&mut[u8])
{
    assert!(size <= 8);
    write_num_bytes!(size, val, out);
}

pub fn read_u8(buf: &[u8;1]) -> u8
{
    return buf[0];
}

pub fn read_u16(buf: &[u8;2]) -> u16
{
    read_num_bytes!(2, buf, u16)
}

pub fn read_u24(buf: &[u8;3]) -> u32
{
    read_num_bytes!(3, buf, u32)
}

pub fn read_u32(buf: &[u8;4]) -> u32
{
    read_num_bytes!(4, buf, u32)
}

pub fn read_u40(buf: &[u8;5]) -> u64
{
    read_num_bytes!(5, buf, u64)
}

pub fn read_u48(buf: &[u8;6]) -> u64
{
    read_num_bytes!(6, buf, u64)
}

pub fn read_u56(buf: &[u8;7]) -> u64
{
    read_num_bytes!(7, buf, u64)
}

pub fn read_u64(buf: &[u8;8]) -> u64
{
    read_num_bytes!(8, buf, u64)
}

pub fn read_to_u64(size:usize, buf:&[u8]) -> u64
{
    assert!(size <= 8);
    read_num_bytes!(size, buf, u64)
}


pub trait Serializable {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError>;
    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError>;
}

pub trait Buffer {
    fn write<T:Serializable>(&mut self, object: &T) -> Result<usize, SerialError>;

    fn read<T:Serializable>(&self) -> Result<T, SerialError>;
}

impl<const N:usize> Serializable for [u8; N] {
    fn serial(&self, out: &mut [u8]) -> Result<usize, SerialError> {
        assert!(N <= out.len());
        unsafe {
            copy_nonoverlapping(self.as_ptr(), out.as_mut_ptr(), N);
        }
        Ok(N)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        assert!(N <= buf.len());
        unsafe {
            copy_nonoverlapping(buf.as_ptr(), self.as_mut_ptr(), N);
        }
        Ok(N)
    }
}

impl Serializable for u8 {
    fn serial(&self, out: &mut [u8]) -> Result<usize, SerialError> {
        match out.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                write_u8(*self, arr);
                Ok(1)
            }
        }
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        match buf.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                *self = read_u8(arr);
                Ok(1)
            }
        }
    }
}

impl Serializable for u16 {
    fn serial(&self, out: &mut [u8]) -> Result<usize, SerialError> {
        match out.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                write_u16(*self, arr);
                Ok(2)
            }
        }
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        match buf.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                *self = read_u16(arr);
                Ok(2)
            }
        }
    }
}

impl Serializable for u32 {
    fn serial(&self, out: &mut [u8]) -> Result<usize, SerialError> {
        match out.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                write_u32(*self, arr);
                Ok(4)
            }
        }
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        match buf.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                *self = read_u32(arr);
                Ok(4)
            }
        }
    }
}

impl Serializable for u64 {
    fn serial(&self, out: &mut [u8]) -> Result<usize, SerialError> {
        match out.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                write_u64(*self, arr);
                Ok(8)
            }
        }
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        match buf.try_into() {
            Err(_) => Err(SerialError{}),
            Ok(arr) => {
                *self = read_u64(arr);
                Ok(8)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{Serializable, SerialError};

    macro_rules! test_serial {
        ($what:expr, $size:expr, $expected:expr) => ({
            let mut buf: [u8;$size] = [0xFF; $size];
            assert_eq!($what.serial(&mut buf[..]), Ok($size));
            assert_eq!(buf, $expected);
        });
    }

    #[test]
    fn serial_u8() {
        test_serial!(0_u8, 1, [0x00]);
        test_serial!(1_u8, 1, [0x01]);
        test_serial!(255_u8, 1, [0xff]);
        test_serial!(128_u8, 1, [0x80]);
    }

    #[test]
    fn serial_u16() {
        test_serial!(0_u16, 2, [0x00, 0x00]);
        test_serial!(1_u16, 2, [0x00, 0x01]);
        test_serial!(255_u16, 2, [0x00, 0xff]);
        test_serial!(128_u16, 2, [0x00, 0x80]);
        test_serial!(0x8000_u16, 2, [0x80, 0x00]);
        test_serial!(0xFF00_u16, 2, [0xFF, 0x00]);
        test_serial!(0xFFFF_u16, 2, [0xFF, 0xFF]);

        let mut buf: [u8;1] = [0xFF;1];
        assert_eq!(1_u16.serial(&mut buf[..]), Err(SerialError{}));
     }

    #[test]
    fn serial_u32() {
        test_serial!(0_u32, 4, [0x00, 0x00, 0x00, 0x00]);
        test_serial!(1_u32, 4, [0x00, 0x00, 0x00, 0x01]);
        test_serial!(255_u32, 4, [0x00, 0x00, 0x00, 0xff]);
        test_serial!(128_u32, 4, [0x00, 0x00, 0x00, 0x80]);
        test_serial!(0x8000_u32, 4, [0x00, 0x00, 0x80, 0x00]);
        test_serial!(0xFF00_u32, 4, [0x00, 0x00, 0xFF, 0x00]);
        test_serial!(0xFFFF_u32, 4, [0x00, 0x00, 0xFF, 0xFF]);
        test_serial!(0xFF000000_u32, 4, [0xFF, 0x00, 0x00, 0x00]);
        test_serial!(0xFF0000_u32, 4, [0x00, 0xFF, 0x00, 0x00]);
        test_serial!(0xFFFFFFFF_u32, 4, [0xFF, 0xFF, 0xFF, 0xFF]);

        let mut buf: [u8;2] = [0xFF;2];
        assert_eq!(1_u32.serial(&mut buf[..]), Err(SerialError{}));
      }

    #[test]
    fn serial_u64() {
        test_serial!(0_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(1_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
        test_serial!(0xFF_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF]);
        test_serial!(0xFF00_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00]);
        test_serial!(0xFF0000_u64, 8, [0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00]);
        test_serial!(0xFF000000_u64, 8, [0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00]);
        test_serial!(0xFF00000000_u64, 8, [0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(0xFF0000000000_u64, 8, [0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(0xFF000000000000_u64, 8, [0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(0xFF00000000000000_u64, 8, [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        test_serial!(0xFF00000000008000_u64, 8, [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00]);
        test_serial!(0xFFFFFFFFFFFFFFFF_u64, 8, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

        let mut buf: [u8;2] = [0xFF;2];
        assert_eq!(1_u64.serial(&mut buf[..]), Err(SerialError{}));
       }

    macro_rules! test_parse {
        ($ty:ty, $buf:expr, $size:expr, $expected:expr) => ({
            let mut data: $ty = 0x88;
            assert_eq!(data.parse(&$buf), Ok($size));
            assert_eq!(data, $expected);
        });
    }

    #[test]
    fn parse_u8() {
        test_parse!(u8, [0x00], 1, 0);
        test_parse!(u8, [0x01], 1, 1);
        test_parse!(u8, [0xff], 1, 255);
        test_parse!(u8, [0x80], 1, 128);
    }

    #[test]
    fn parse_u16() {
        test_parse!(u16, [0x00, 0x00], 2, 0);
        test_parse!(u16, [0x00, 0x01], 2, 1);
        test_parse!(u16, [0x00, 0xff], 2, 255);
        test_parse!(u16, [0x00, 0x80], 2, 128);
        test_parse!(u16, [0x80, 0x00], 2, 0x8000);
        test_parse!(u16, [0xFF, 0x00], 2, 0xFF00);
        test_parse!(u16, [0xFF, 0xFF], 2, 0xFFFF);
     }

    #[test]
    fn parse_u32() {
        test_parse!(u32, [0x00, 0x00, 0x00, 0x00], 4, 0);
        test_parse!(u32, [0x00, 0x00, 0x00, 0x01], 4, 1);
        test_parse!(u32, [0x00, 0x00, 0x00, 0xff], 4, 255);
        test_parse!(u32, [0x00, 0x00, 0x00, 0x80], 4, 128);
        test_parse!(u32, [0x00, 0x00, 0x80, 0x00], 4, 0x8000);
        test_parse!(u32, [0x00, 0x00, 0xFF, 0x00], 4, 0xFF00);
        test_parse!(u32, [0x00, 0x00, 0xFF, 0xFF], 4, 0xFFFF);
        test_parse!(u32, [0xFF, 0x00, 0x00, 0x00], 4, 0xFF000000);
        test_parse!(u32, [0x00, 0xFF, 0x00, 0x00], 4, 0xFF0000);
        test_parse!(u32, [0xFF, 0xFF, 0xFF, 0xFF], 4, 0xFFFFFFFF);
     }

    #[test]
    fn parse_u64() {
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 8, 0);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], 8, 1);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF], 8, 0xFF);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00], 8, 0xFF00);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00], 8, 0xFF0000);
        test_parse!(u64, [0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00], 8, 0xFF000000);
        test_parse!(u64, [0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00], 8, 0xFF00000000);
        test_parse!(u64, [0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00], 8, 0xFF0000000000);
        test_parse!(u64, [0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 8, 0xFF000000000000);
        test_parse!(u64, [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 8, 0xFF00000000000000);
        test_parse!(u64, [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00], 8, 0xFF00000000008000);
        test_parse!(u64, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], 8, 0xFFFFFFFFFFFFFFFF);
      }

}
