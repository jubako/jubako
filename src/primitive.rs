
use std::ptr::copy_nonoverlapping;
use std::mem::size_of;
//use std::mem::{size_of, size_of_val};

/*
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
}*/

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

/*
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
}*/

pub fn read_u8(buf: &[u8]) -> u8
{
    assert!(1 <= buf.len());
    return buf[0];
}

pub fn read_u16(buf: &[u8]) -> u16
{
    read_num_bytes!(2, buf, u16)
}

/*
pub fn read_u24(buf: &[u8]) -> u32
{
    read_num_bytes!(3, buf, u32)
}
*/

pub fn read_u32(buf: &[u8]) -> u32
{
    read_num_bytes!(4, buf, u32)
}

/*
pub fn read_u40(buf: &[u8]) -> u64
{
    read_num_bytes!(5, buf, u64)
}

pub fn read_u48(buf: &[u8]) -> u64
{
    read_num_bytes!(6, buf, u64)
}

pub fn read_u56(buf: &[u8]) -> u64
{
    read_num_bytes!(7, buf, u64)
}
*/
pub fn read_u64(buf: &[u8]) -> u64
{
    read_num_bytes!(8, buf, u64)
}

pub fn read_to_u64(size:usize, buf:&[u8]) -> u64
{
    assert!(size <= 8);
    read_num_bytes!(size, buf, u64)
}
