use crate::bases::*;

/// Store the base array we can found in entry property.
///
/// When storing a array in a entry, the array is stored in a ValueStore.
/// But user can decide to store a fixed prefix directly in the entry.
/// BaseArray is a structure to store this prefix, at reading time.
/// As the prefix cannot be longer than 31, BaseArray is wrapper around a 31 bytes length array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct BaseArray {
    pub data: [u8; 31],
}

impl BaseArray {
    /// Create a BaseArry from `data`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let array = jubako::bases::BaseArray::new(&[0,5,12]);
    /// assert_eq!(array.data, [0, 5, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    /// ```
    ///
    /// # Panics
    ///
    /// The function panics if `data` length is higher than 31.
    pub fn new(data: &[u8]) -> Self {
        assert!(data.len() <= 31);
        let mut s = Self { data: [0; 31] };
        if !data.is_empty() {
            s.data[..data.len()].copy_from_slice(data);
        }
        s
    }

    /// Create a BaseArray taking `size` bytes from `flux`.
    ///
    /// # Panics
    ///
    /// The function panics if `size` is higher than 31.
    ///
    /// # Error
    ///
    /// This function will return an error if reading from `flux` fails.
    pub fn new_from_flux(size: u8, flux: &mut Flux) -> Result<Self> {
        assert!(size <= 31);
        let mut s = Self { data: [0; 31] };
        if size != 0 {
            flux.read_exact(&mut s.data[..size as usize])?;
        }
        Ok(s)
    }
}
