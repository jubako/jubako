use crate::io::*;

/// This module contains all base headers.

macro_rules! Struct {
    (@memberdef
     ($structname:ident : field
      $member:ident : $type:ty => offset:$offset:expr,
      $($tail:tt)* )
     ->
     ($($output:tt)*)
    ) =>
    {
        Struct!{@memberdef ($structname : $($tail)*) -> ($($output)* $member: $type,)}
    };
    (@memberdef
     ($structname:ident : padding
      size:$size:expr => offset:$offset:expr,
      $($tail:tt)* )
     ->
     ($($output:tt)*)
    ) =>
    {
        Struct!{@memberdef ($structname : $($tail)*) -> ($($output)*)}
    };
    (@memberdef
     ($structname:ident :)
     ->
     ($($output:tt)*)
    ) => {
        Struct!{@asstruct $structname, $($output)*}
    };

    (@asstruct $structname:ident, $($body:tt)*) => {
        pub struct $structname {
            $($body)*
        }
    };

    (@serialmember $self_:ident, $out:ident,) => {};
    (@serialmember $self_:ident, $out:ident, field $member:ident : $type:ty => offset:$offset:expr, $($tail:tt)* ) => {
        $self_.$member.serial(&mut $out[$offset..])?;
        Struct!(@serialmember $self_, $out, $($tail)*);
    };
    (@serialmember $self_:ident, $out:ident, padding size:$size:expr => offset:$offset:expr, $($tail:tt)* ) => {
        [0_u8; $size].serial(&mut $out[$offset..])?;
        Struct!(@serialmember $self_, $out, $($tail)*);
    };
    (@serialmethod $self_:ident $size:expr, $($members:tt)+) => {
        fn serial(&$self_, out: &mut[u8]) -> Result<usize, SerialError> {
            Struct!(@serialmember $self_, out, $($members)+);
            Ok($size)
        }
    };

    (@parsemember $self_:ident, $buf:ident,) => {};
    (@parsemember $self_:ident, $buf:ident, field $member:ident : $type:ty => offset:$offset:expr, $($tail:tt)* ) => {
        $self_.$member.parse(&$buf[$offset..])?;
        Struct!(@parsemember $self_, $buf, $($tail)*);
    };
    (@parsemember $self_:ident, $buf:ident, padding size:$size:expr => offset:$offset:expr, $($tail:tt)* ) => {
        Struct!(@parsemember $self_, $buf, $($tail)*);
    };
    (@parsemethod $self_:ident $size:expr, $($members:tt)+) => {
        fn parse(&mut $self_, buf: &[u8]) -> Result<usize, SerialError> {
            Struct!(@parsemember $self_, buf, $($members)+);
            Ok($size)
        }
    };

    ($structname:ident, $size:expr => $($members:tt)+ ) => {
        Struct!{@memberdef ($structname : $($members)+,) -> ()}
        impl Serializable for $structname {
            Struct!{@serialmethod self $size, $($members)+,}
            Struct!{@parsemethod self $size, $($members)+,}
        }
    }
}

Struct!{
    PackHeader, 48 =>
        field magic:u32 => offset:0,
        field app_vendor_id:u32 => offset:4,
        field major_version:u8 => offset:8,
        field minor_version:u8 => offset:9,
        field uuid:[u8;16] => offset:10,
        padding size:6 => offset:26,
        field file_size:u64 => offset:32,
        field check_info_pos:u64 => offset:40
}

Struct!{
    ArxHeader, 16 =>
        field pack_count:u8 => offset:0,
        field free_data:[u8;15] => offset:1
}

Struct!{
    PackInfoHeader, 256 =>
        field uuid:[u8;16] => offset:0,
        field id:u8 => offset:16,
        field free_data:[u8;103] => offset:17,
        field size:u64 => offset:120,
        field offset:u64 => offset:128,
        field check_info_pos:u64 => offset:136,
        field path: [u8;112] => offset:144
}

Struct!{
    DirectoryPackHeader, 80 =>
        field entry_store_ptr_pos:u64 => offset:0,
        field key_store_ptr_pos:u64 => offset:8,
        field index_ptr_pos:u64 => offset:16,
        field index_count:u32 => offset:24,
        field entry_store_count:u32 => offset:28,
        field key_store_count:u8 => offset:32,
        field free_data:[u8;47] => offset:33
}

Struct!{
    ContentPackHeader, 80 =>
        field entry_ptr_pos:u64 => offset:0,
        field cluster_ptr_pos:u64 => offset:8,
        field entry_count:u32 => offset:16,
        field cluster_count:u32 => offset:20,
        field free_data:[u8;56] => offset:24
}

pub struct EntryInfo {
    cluster_index: u32,
    blob_index: u16
}
impl Serializable for EntryInfo {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError> {
        let mut data: u32 = self.cluster_index << 12;
        data += (self.blob_index & 0xFFF) as u32;
        data.serial(&mut out[..])?;
        Ok(4)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        self.cluster_index.parse(&buf[..])?;
        self.blob_index = (self.cluster_index & 0xFFF) as u16;
        self.cluster_index >>= 12;
        Ok(4)
    }
}

Struct!{
    ClusterHeader, 12 =>
        field typ:u8 => offset:0,
        field offset_size:u8 => offset:1,
        field blob_count:u16 => offset:2,
        field cluster_size:u64 => offset:4
        // Followed by dataSize, offsets and data.
}

struct EntryStoreHeader<T:Serializable> {
    typ: u8,
    header_size: u16,
    header_data: T,
    entry_data_size: u64,
}

impl<T:Serializable> Serializable for EntryStoreHeader<T> {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError> {
        self.typ.serial(&mut out[..])?;
        self.header_size.serial(&mut out[1..])?;
        let written = self.header_data.serial(&mut out[3..])?;
        self.entry_data_size.serial(&mut out[3+written..])?;
        Ok(3+8+written)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        self.typ.parse(&buf[..])?;
        self.header_size.parse(&buf[1..])?;
        let read = self.header_data.parse(&buf[3..])?;
        self.entry_data_size.parse(&buf[3+read..])?;
        Ok(3+8+read)
    }
}

struct PlainStoreDataHeader {
    entry_size: u8,
    variant_count: u8,
    key_count: u8
    // variant Def
}

impl Serializable for PlainStoreDataHeader {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError> {
        self.entry_size.serial(&mut out[..])?;
        self.variant_count.serial(&mut out[1..])?;
        self.key_count.serial(&mut out[2..])?;
        // Serial key_count keys (key_count * u8)
        Ok(3_usize+self.key_count as usize)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        self.entry_size.parse(&buf[..])?;
        self.variant_count.parse(&buf[1..])?;
        self.key_count.parse(&buf[2..])?;
        Ok(3_usize+self.key_count as usize)
    }
}

Struct!{
    RefStoreDataHeader, 5 =>
        padding size:1 => offset:0,
        field base_store_index:u32 => offset:1
}

Struct!{
    FullRefStoreDataHeader, 1 =>
        padding size:1 => offset:0
}

Struct!{
    PlainKeyStoreHeader, 16 =>
        field typ: u8 => offset:0,
        padding size:7 => offset:1,
        field key_data_size:u64 => offset:8
}

struct IndexedKeyStoreHeader {
    typ: u8,
    size: u64,
    entry_count: u64,
    offset_size: u8,
    key_data_size: u64,
    // [TODO] offsets
}

impl Serializable for IndexedKeyStoreHeader {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError> {
        self.typ.serial(&mut out[..])?;
        self.size.serial(&mut out[1..])?;
        self.entry_count.serial(&mut out[9..])?;
        self.offset_size.serial(&mut out[17..])?;
        write_from_u64(self.key_data_size, self.offset_size as usize, &mut out[18..]);
        Ok(18_usize+self.offset_size as usize)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        self.typ.parse(&buf[..])?;
        self.size.parse(&buf[1..])?;
        self.entry_count.parse(&buf[9..])?;
        self.offset_size.parse(&buf[17..])?;
        self.key_data_size = read_to_u64(self.offset_size as usize, &buf[18..]);
        Ok(18_usize+self.offset_size as usize)
    }
}

pub struct ContentAddress {
    cluster_index: u32,
    blob_index: u16
}


impl Serializable for ContentAddress {
    fn serial(&self, out: &mut[u8]) -> Result<usize, SerialError> {
        let mut data: u32 = self.cluster_index << 12;
        data += (self.blob_index & 0xFFF) as u32;
        data.serial(&mut out[..])?;
        Ok(4)
    }

    fn parse(&mut self, buf: &[u8]) -> Result<usize, SerialError> {
        self.cluster_index.parse(&buf[..])?;
        self.blob_index = (self.cluster_index & 0xFFF) as u16;
        self.cluster_index >>= 12;
        Ok(4)
    }
}

Struct!{
    IndexHeader, 20 =>
        field header_size:u16 => offset:0,
        field index_key:u8 => offset:2,
        padding size:1 => offset:3,
        field store_id:u32 => offset:4,
        field entry_count:u32 => offset:8,
        field entry_offset:u32 => offset:12,
        field extra_data:ContentAddress => offset:16
        //field index_name:PString => offset:20
}
