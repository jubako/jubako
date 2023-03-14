use crate::bases::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BaseArray {
    pub data: [u8; 31],
}

impl BaseArray {
    pub fn new(data: &[u8]) -> Self {
        assert!(data.len() <= 31);
        let mut s = Self { data: [0; 31] };
        if !data.is_empty() {
            s.data[..data.len()].copy_from_slice(data);
        }
        s
    }

    pub fn new_from_flux(size: u8, flux: &mut Flux) -> Result<Self> {
        assert!(size <= 31);
        let mut s = Self { data: [0; 31] };
        if size != 0 {
            flux.read_exact(&mut s.data[..size as usize])?;
        }
        Ok(s)
    }
}
