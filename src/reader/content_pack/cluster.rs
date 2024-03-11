use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType};
use serde::ser::SerializeStruct;
use std::sync::{Arc, RwLock};

enum ClusterReader {
    // The reader on the raw data as stored in the cluster.
    Raw(Reader),
    // The reader on the plain data as we need to read it.
    // May be the same that a raw reader if the data is not compressed
    Plain(Reader),
}

pub struct Cluster {
    blob_offsets: Vec<Offset>,
    data_size: Size,
    compression: CompressionType,
    reader: RwLock<ClusterReader>,
}

#[cfg(feature = "lz4")]
fn lz4_source(raw_stream: Stream, data_size: Size) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        lz4::Decoder::new(raw_stream)?,
        data_size,
    )))
}

#[cfg(not(feature = "lz4"))]
fn lz4_source(_raw_stream: Stream, _data_size: Size) -> Result<Arc<dyn Source>> {
    Err("Lz4 compression is not supported in this configuration."
        .to_string()
        .into())
}

#[cfg(feature = "lzma")]
fn lzma_source(raw_stream: Stream, data_size: Size) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        xz2::read::XzDecoder::new_stream(
            raw_stream,
            xz2::stream::Stream::new_lzma_decoder(128 * 1024 * 1024)?,
        ),
        data_size,
    )))
}

#[cfg(not(feature = "lzma"))]
fn lzma_source(_raw_stream: Stream, _data_size: Size) -> Result<Arc<dyn Source>> {
    Err("Lzma compression is not supported in this configuration."
        .to_string()
        .into())
}

#[cfg(feature = "zstd")]
fn zstd_source(raw_stream: Stream, data_size: Size) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        zstd::Decoder::new(raw_stream)?,
        data_size,
    )))
}

#[cfg(not(feature = "zstd"))]
fn zstd_source(_raw_stream: Stream, _data_size: Size) -> Result<Arc<dyn Source>> {
    Err("zstd compression is not supported in this configuration."
        .to_string()
        .into())
}

impl Cluster {
    pub fn new(reader: &Reader, cluster_info: SizedOffset) -> Result<Self> {
        let header_reader =
            reader.create_sub_memory_reader(cluster_info.offset, End::Size(cluster_info.size))?;
        let mut flux = header_reader.create_flux_all();
        let header = ClusterHeader::produce(&mut flux)?;
        let raw_data_size: Size = flux.read_usized(header.offset_size)?.into();
        let data_size: Size = flux.read_usized(header.offset_size)?.into();
        let blob_count = header.blob_count.into_usize();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity(blob_count + 1);
        let uninit = blob_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..blob_count] {
            let value: Offset = if first {
                first = false;
                Offset::zero()
            } else {
                flux.read_usized(header.offset_size)?.into()
            };
            assert!(value.is_valid(data_size));
            elem.write(value);
        }
        unsafe { blob_offsets.set_len(blob_count) }
        blob_offsets.push(data_size.into());
        let reader = if header.compression == CompressionType::None {
            if raw_data_size != data_size {
                return Err(format_error!(
                    &format!(
                        "Stored size ({raw_data_size}) must be equal to data size ({data_size}) if no comprresion."
                    ),
                    flux
                ));
            }
            ClusterReader::Plain(
                reader
                    .create_sub_reader(
                        cluster_info.offset - raw_data_size,
                        End::Size(raw_data_size),
                    )
                    .into(),
            )
        } else {
            ClusterReader::Raw(
                reader
                    .create_sub_reader(
                        cluster_info.offset - raw_data_size,
                        End::Size(raw_data_size),
                    )
                    .into(),
            )
        };
        Ok(Cluster {
            blob_offsets,
            data_size,
            compression: header.compression,
            reader: RwLock::new(reader),
        })
    }

    fn build_plain_reader(&self) -> Result<()> {
        let mut cluster_reader = self.reader.write().unwrap();
        if let ClusterReader::Plain(_) = *cluster_reader {
            return Ok(());
        };

        let raw_reader = if let ClusterReader::Raw(r) = &*cluster_reader {
            r.create_sub_reader(Offset::zero(), End::None)
        } else {
            unreachable!()
        };
        let raw_flux = raw_reader.create_flux_all();
        let decompress_reader = match self.compression {
            CompressionType::Lz4 => Reader::new_from_arc(
                lz4_source(raw_flux.into(), self.data_size)?,
                End::Size(self.data_size),
            ),
            CompressionType::Lzma => Reader::new_from_arc(
                lzma_source(raw_flux.into(), self.data_size)?,
                End::Size(self.data_size),
            ),
            CompressionType::Zstd => Reader::new_from_arc(
                zstd_source(raw_flux.into(), self.data_size)?,
                End::Size(self.data_size),
            ),
            CompressionType::None => unreachable!(),
        };
        *cluster_reader = ClusterReader::Plain(decompress_reader);
        Ok(())
    }

    #[cfg(test)]
    fn blob_count(&self) -> BlobCount {
        BlobCount::from((self.blob_offsets.len() - 1) as u16)
    }

    pub fn get_reader(&self, index: BlobIdx) -> Result<Reader> {
        self.build_plain_reader()?;
        let offset = self.blob_offsets[index.into_usize()];
        let end_offset = self.blob_offsets[index.into_usize() + 1];
        if let ClusterReader::Plain(r) = &*self.reader.read().unwrap() {
            Ok(r.create_sub_reader(offset, End::Offset(end_offset)).into())
        } else {
            unreachable!()
        }
    }
}

impl serde::Serialize for Cluster {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut cont = serializer.serialize_struct("Cluster", 3)?;
        cont.serialize_field("offset", &(self.blob_offsets.len() - 1))?;
        cont.serialize_field("size", &self.data_size)?;
        cont.serialize_field("compression", &self.compression)?;
        cont.end()
    }
}

impl Explorable for Cluster {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        let (item, decode) = if let Some(item) = item.strip_suffix('#') {
            (item, true)
        } else {
            (item, false)
        };
        let index = item
            .parse::<u16>()
            .map_err(|e| Error::from(format!("{e}")))?;

        if index >= (self.blob_offsets.len() as u16 - 1) {
            return Ok(None);
        }
        let reader = self.get_reader(BlobIdx::from(index))?;
        let data = reader
            .create_flux_all()
            .read_vec(reader.size().into_usize())?;
        Ok(Some(if decode {
            Box::new(String::from_utf8_lossy(&data).into_owned())
        } else {
            Box::new(data)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;
    use test_case::test_case;

    fn create_cluster(comp: CompressionType, data: &[u8]) -> (SizedOffset, Vec<u8>) {
        let mut cluster_data = Vec::new();
        cluster_data.extend_from_slice(&data);
        #[rustfmt::skip]
        cluster_data.extend_from_slice(&[
            comp as u8,       // compression
            0x01,             // offset_size
            0x03, 0x00,       // blob_count
            data.len() as u8, // raw data size
            0x0f,             // Data size
            0x05,             // Offset of blob 1
            0x08,             // Offset of blob 2
        ]);
        (
            SizedOffset::new(
                Size::from(cluster_data.len() - data.len()),
                Offset::from(data.len()),
            ),
            cluster_data,
        )
    }

    fn create_raw_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        let raw_data = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        Some(create_cluster(CompressionType::None, &raw_data))
    }

    #[cfg(feature = "lz4")]
    fn create_lz4_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder = lz4::EncoderBuilder::new()
                .level(16)
                .build(Cursor::new(compressed_content))
                .unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            let (compressed_content, err) = encoder.finish();
            err.unwrap();
            compressed_content.into_inner()
        };
        Some(create_cluster(CompressionType::Lz4, &data))
    }

    #[cfg(not(feature = "lz4"))]
    fn create_lz4_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        None
    }

    #[cfg(feature = "lzma")]
    fn create_lzma_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder = xz2::write::XzEncoder::new_stream(
                Cursor::new(compressed_content),
                xz2::stream::Stream::new_lzma_encoder(
                    &xz2::stream::LzmaOptions::new_preset(9).unwrap(),
                )
                .unwrap(),
            );
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        Some(create_cluster(CompressionType::Lzma, &data))
    }

    #[cfg(not(feature = "lzma"))]
    fn create_lzma_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        None
    }

    #[cfg(feature = "zstd")]
    fn create_zstd_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder = zstd::Encoder::new(Cursor::new(compressed_content), 0).unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        Some(create_cluster(CompressionType::Zstd, &data))
    }

    #[cfg(not(feature = "zstd"))]
    fn create_zstd_cluster() -> Option<(SizedOffset, Vec<u8>)> {
        None
    }

    type ClusterCreator = fn() -> Option<(SizedOffset, Vec<u8>)>;

    #[test_case(CompressionType::None, create_raw_cluster)]
    #[test_case(CompressionType::Lz4, create_lz4_cluster)]
    #[test_case(CompressionType::Lzma, create_lzma_cluster)]
    #[test_case(CompressionType::Zstd, create_zstd_cluster)]
    fn test_cluster(comp: CompressionType, creator: ClusterCreator) {
        let cluster_info = creator();
        if cluster_info.is_none() {
            return;
        }
        let (ptr_info, data) = cluster_info.unwrap();
        let reader = Reader::from(data);
        let mut flux = reader.create_flux_from(ptr_info.offset);
        let header = ClusterHeader::produce(&mut flux).unwrap();
        assert_eq!(header.compression, comp);
        assert_eq!(header.offset_size, ByteSize::U1);
        assert_eq!(header.blob_count, 3.into());
        let cluster = Cluster::new(&reader, ptr_info).unwrap();
        assert_eq!(cluster.blob_count(), 3.into());

        {
            let sub_reader = cluster.get_reader(BlobIdx::from(0)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut flux = sub_reader.create_flux_all();
            flux.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let sub_reader = cluster.get_reader(BlobIdx::from(1)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut flux = sub_reader.create_flux_all();
            flux.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let sub_reader = cluster.get_reader(BlobIdx::from(2)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut flux = sub_reader.create_flux_all();
            flux.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
