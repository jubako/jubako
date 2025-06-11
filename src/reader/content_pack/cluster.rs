use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType};
use crate::reader::{ByteRegion, ByteStream};
use std::sync::{Arc, RwLock};

enum ClusterReader {
    // The reader on the raw data as stored in the cluster.
    Raw(Reader),
    // The reader on the plain data as we need to read it.
    // May be the same that a raw reader if the data is not compressed
    Plain(Reader),
}

pub(super) struct Cluster {
    blob_offsets: Vec<Offset>,
    data_size: Size,
    compression: CompressionType,
    reader: RwLock<ClusterReader>,
}

#[cfg(feature = "lz4")]
fn lz4_source(raw_stream: ByteStream, data_size: ASize) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        lz4::Decoder::new(raw_stream)?,
        data_size,
    )))
}

#[cfg(not(feature = "lz4"))]
fn lz4_source(_raw_stream: ByteStream, _data_size: ASize) -> Result<Arc<dyn Source>> {
    Err(MissingFeatureError {
        name: "lz4",
        msg: "Lz4 compression is not supported in this configuration.",
    }
    .into())
}

#[cfg(feature = "lzma")]
fn lzma_source(raw_stream: ByteStream, data_size: ASize) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        xz2::read::XzDecoder::new_stream(
            raw_stream,
            xz2::stream::Stream::new_lzma_decoder(128 * 1024 * 1024)?,
        ),
        data_size,
    )))
}

#[cfg(not(feature = "lzma"))]
fn lzma_source(_raw_stream: ByteStream, _data_size: ASize) -> Result<Arc<dyn Source>> {
    Err(MissingFeatureError {
        name: "lzma",
        msg: "Lzma compression is not supported in this configuration.",
    }
    .into())
}

#[cfg(feature = "zstd")]
fn zstd_source(raw_stream: ByteStream, data_size: ASize) -> Result<Arc<dyn Source>> {
    Ok(Arc::new(SeekableDecoder::new(
        zstd::Decoder::new(raw_stream)?,
        data_size,
    )))
}

#[cfg(not(feature = "zstd"))]
fn zstd_source(_raw_stream: ByteStream, _data_size: ASize) -> Result<Arc<dyn Source>> {
    Err(MissingFeatureError {
        name: "zstd",
        msg: "zstd compression is not supported in this configuration.",
    }
    .into())
}

impl Cluster {
    fn build_plain_reader(&self) -> Result<()> {
        let mut cluster_reader = self.reader.write().unwrap();
        if let ClusterReader::Plain(_) = *cluster_reader {
            return Ok(());
        };

        let raw_stream = if let ClusterReader::Raw(r) = &*cluster_reader {
            r.create_stream(Offset::zero(), r.size(), false)?
        } else {
            unreachable!()
        };
        let decompress_reader = match self.compression {
            CompressionType::Lz4 => Reader::new_from_arc(
                lz4_source(raw_stream, (self.data_size.into_u64() as usize).into())?,
                self.data_size,
            ),
            CompressionType::Lzma => Reader::new_from_arc(
                lzma_source(raw_stream, (self.data_size.into_u64() as usize).into())?,
                self.data_size,
            ),
            CompressionType::Zstd => Reader::new_from_arc(
                zstd_source(raw_stream, (self.data_size.into_u64() as usize).into())?,
                self.data_size,
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

    pub fn get_bytes(&self, index: BlobIdx) -> Result<ByteRegion> {
        self.build_plain_reader()?;
        let offset = self.blob_offsets[index.into_usize()];
        let end_offset = self.blob_offsets[index.into_usize() + 1];
        let size = end_offset - offset;
        if let ClusterReader::Plain(r) = &*self.reader.read().unwrap() {
            Ok(r.get_byte_slice(offset, size).into())
        } else {
            unreachable!()
        }
    }
}

pub(crate) struct ClusterBuilder {
    blob_offsets: Vec<Offset>,
    data_size: Size,
    compression: CompressionType,
}

impl DataBlockParsable for Cluster {
    type TailParser = ClusterBuilder;
    type Output = Self;

    fn finalize(
        intermediate: (ClusterBuilder, Size),
        header_offset: Offset,
        reader: &Reader,
    ) -> Result<Self::Output> {
        let (cluster_builder, raw_data_size) = intermediate;
        let reader = reader.cut(header_offset - raw_data_size, raw_data_size, false)?;
        let reader = if cluster_builder.compression == CompressionType::None {
            assert_eq!(cluster_builder.data_size, raw_data_size);
            ClusterReader::Plain(reader)
        } else {
            assert!(raw_data_size.into_u64() <= usize::MAX as u64);
            ClusterReader::Raw(reader)
        };
        Ok(Cluster {
            blob_offsets: cluster_builder.blob_offsets,
            data_size: cluster_builder.data_size,
            compression: cluster_builder.compression,
            reader: RwLock::new(reader),
        })
    }
}

impl Parsable for ClusterBuilder {
    type Output = (ClusterBuilder, Size);
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        let header = ClusterHeader::parse(parser)?;
        let raw_data_size: Size = parser.read_usized(header.offset_size)?.into();
        let data_size: Size = parser.read_usized(header.offset_size)?.into();
        let blob_count = header.blob_count.into_usize();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity(blob_count + 1);
        let uninit = blob_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..blob_count] {
            let value: Offset = if first {
                first = false;
                Offset::zero()
            } else {
                parser.read_usized(header.offset_size)?.into()
            };
            assert!(value.is_valid(data_size));
            elem.write(value);
        }
        unsafe { blob_offsets.set_len(blob_count) }
        blob_offsets.push(data_size.into());
        if header.compression == CompressionType::None && raw_data_size != data_size {
            return Err(format_error!(
                    &format!(
                        "Stored size ({raw_data_size}) must be equal to data size ({data_size}) if no comprresion."
                    ),
                    parser
                ));
        }

        Ok((
            ClusterBuilder {
                blob_offsets,
                data_size,
                compression: header.compression,
            },
            raw_data_size,
        ))
    }
}

impl BlockParsable for ClusterBuilder {}

#[cfg(feature = "explorable_serde")]
impl serde::Serialize for Cluster {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut cont = serializer.serialize_struct("Cluster", 3)?;
        cont.serialize_field("offset", &(self.blob_offsets.len() - 1))?;
        cont.serialize_field("size", &self.data_size)?;
        cont.serialize_field("compression", &self.compression)?;
        cont.end()
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for Cluster {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("Cluster(".to_string(), ")".to_string()))
    }

    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        out.field(
            &format!("blobs count ({} or {})", "<N>".bold(), "<N>#".bold()),
            &(self.blob_offsets.len() - 1),
        )?;
        out.field("size", &self.data_size)?;
        out.field("compression", &self.compression)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for Cluster {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        let (key, pretty_print) = if key.ends_with('#') {
            (key.split_at(key.len() - 1).0, true)
        } else {
            (key, false)
        };

        let index = key
            .parse::<u16>()
            .map_err(|_e| graphex::Error::Key(key.to_string()))?;

        if index >= (self.blob_offsets.len() as u16 - 1) {
            return Err(graphex::Error::key(key));
        }
        let bytes = self.get_bytes(BlobIdx::from(index))?;

        if pretty_print {
            let size = std::cmp::min(bytes.size().into_u64(), 0xFFFF) as usize;
            let slice = bytes.get_slice(Offset::zero(), size)?;
            Ok(Box::new(String::from_utf8_lossy(&slice).into_owned()).into())
        } else {
            Ok(Box::new(bytes).into())
        }
    }

    fn display(&self) -> &dyn graphex::Display {
        self
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Read;

    fn create_cluster(comp: CompressionType, data: &[u8]) -> (SizedOffset, Vec<u8>) {
        let mut cluster_data = Vec::new();
        cluster_data.extend_from_slice(data);
        #[rustfmt::skip]
        let cluster_header = [
            comp as u8,       // compression
            0x01,             // offset_size
            0x03, 0x00,       // blob_count
            data.len() as u8, // raw data size
            0x0f,             // Data size
            0x05,             // Offset of blob 1
            0x08,             // Offset of blob 2
        ];
        cluster_data.extend_from_slice(&cluster_header);
        let mut digest = CRC.digest();
        digest.update(&cluster_header);
        let checksum = digest.finalize().to_be_bytes();
        cluster_data.extend_from_slice(&checksum);
        (
            SizedOffset::new(
                ASize::from(cluster_data.len() - data.len() - 4),
                Offset::from(data.len()),
            ),
            cluster_data,
        )
    }

    fn create_raw_cluster() -> (SizedOffset, Vec<u8>) {
        let raw_data = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        create_cluster(CompressionType::None, &raw_data)
    }

    #[cfg(feature = "lz4")]
    fn create_lz4_cluster() -> (SizedOffset, Vec<u8>) {
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
        create_cluster(CompressionType::Lz4, &data)
    }

    #[cfg(feature = "lzma")]
    fn create_lzma_cluster() -> (SizedOffset, Vec<u8>) {
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
        create_cluster(CompressionType::Lzma, &data)
    }

    #[cfg(feature = "zstd")]
    fn create_zstd_cluster() -> (SizedOffset, Vec<u8>) {
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
        create_cluster(CompressionType::Zstd, &data)
    }

    type ClusterCreator = fn() -> (SizedOffset, Vec<u8>);

    #[derive(Clone, Copy)]
    pub struct TestComp(CompressionType);

    impl From<CompressionType> for TestComp {
        fn from(v: CompressionType) -> Self {
            Self(v)
        }
    }

    impl rustest::ParamName for TestComp {
        fn param_name(&self) -> String {
            format!("{:?}", self.0)
        }
    }

    #[rustest::fixture(params:TestComp = [
        TestComp(CompressionType::None),
        #[cfg(feature = "lz4")]
        TestComp(CompressionType::Lz4),
        #[cfg(feature = "lzma")]
        TestComp(CompressionType::Lzma),
        #[cfg(feature = "zstd")]
        TestComp(CompressionType::Zstd)
    ])]
    fn Compression(Param(TestComp(comp)): Param) -> (CompressionType, ClusterCreator) {
        (
            comp,
            match comp {
                CompressionType::None => create_raw_cluster,
                #[cfg(feature = "lz4")]
                CompressionType::Lz4 => create_lz4_cluster,
                #[cfg(feature = "lzma")]
                CompressionType::Lzma => create_lzma_cluster,
                #[cfg(feature = "zstd")]
                CompressionType::Zstd => create_zstd_cluster,
                _ => unreachable!(),
            },
        )
    }

    #[rustest::test]
    fn test_cluster(comp: Compression) {
        let (comp, creator) = *comp;
        let (ptr_info, data) = creator();
        let reader = CheckReader::from(data);
        let header = reader
            .parse_in::<ClusterHeader>(ptr_info.offset, ptr_info.size)
            .unwrap();
        assert_eq!(header.compression, comp);
        assert_eq!(header.offset_size, ByteSize::U1);
        assert_eq!(header.blob_count, 3.into());

        let reader: Reader = reader.into();
        let cluster = reader.parse_data_block::<Cluster>(ptr_info).unwrap();
        assert_eq!(cluster.blob_count(), 3.into());

        {
            let region = cluster.get_bytes(BlobIdx::from(0)).unwrap();
            assert_eq!(region.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = region.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let region = cluster.get_bytes(BlobIdx::from(1)).unwrap();
            assert_eq!(region.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = region.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let region = cluster.get_bytes(BlobIdx::from(2)).unwrap();
            assert_eq!(region.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = region.stream();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
