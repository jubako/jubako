use crate::bases::*;
use crate::common::{ClusterHeader, CompressionType};
use std::cell::RefCell;

enum ClusterReader {
    // The reader on the raw data as stored in the cluster.
    Raw(Box<dyn Reader>),
    // The reader on the plain data as we need to read it.
    // May be the same that a raw reader if the data is not compressed
    Plain(Box<dyn Reader>),
}

pub struct Cluster {
    blob_offsets: Vec<Offset>,
    data_size: Size,
    compression: CompressionType,
    reader: RefCell<ClusterReader>,
}

impl Cluster {
    pub fn new(reader: &dyn Reader, cluster_info: SizedOffset) -> Result<Self> {
        let header_reader =
            reader.create_sub_memory_reader(cluster_info.offset, End::Size(cluster_info.size))?;
        let mut stream = header_reader.create_stream_all();
        let header = ClusterHeader::produce(stream.as_mut())?;
        let raw_data_size: Size = stream.read_sized(header.offset_size.into())?.into();
        let data_size: Size = stream.read_sized(header.offset_size.into())?.into();
        let blob_count = header.blob_count.into_usize();
        let mut blob_offsets: Vec<Offset> = Vec::with_capacity(blob_count + 1);
        let uninit = blob_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..blob_count] {
            let value: Offset = if first {
                first = false;
                0.into()
            } else {
                stream.read_sized(header.offset_size.into())?.into()
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
                        "Stored size ({}) must be equal to data size ({}) if no comprresion.",
                        raw_data_size, data_size
                    ),
                    stream
                ));
            }
            ClusterReader::Plain(reader.create_sub_reader(
                cluster_info.offset - raw_data_size,
                End::Size(raw_data_size),
            ))
        } else {
            ClusterReader::Raw(reader.create_sub_reader(
                cluster_info.offset - raw_data_size,
                End::Size(raw_data_size),
            ))
        };
        Ok(Cluster {
            blob_offsets,
            data_size,
            compression: header.compression,
            reader: RefCell::new(reader),
        })
    }

    fn build_plain_reader(&self) -> Result<()> {
        let mut cluster_reader = self.reader.borrow_mut();
        if let ClusterReader::Plain(_) = *cluster_reader {
            return Ok(());
        };

        let raw_reader = if let ClusterReader::Raw(r) = &*cluster_reader {
            r.create_sub_reader(Offset(0), End::None)
        } else {
            unreachable!()
        };
        let raw_stream = raw_reader.create_stream_all();
        let decompress_reader = match self.compression {
            CompressionType::Lz4 => Box::new(Lz4Reader::new(
                lz4::Decoder::new(raw_stream)?,
                self.data_size,
            )) as Box<dyn Reader>,
            CompressionType::Lzma => Box::new(LzmaReader::new(
                lzma::LzmaReader::new_decompressor(raw_stream)?,
                self.data_size,
            )),
            CompressionType::Zstd => {
                let reader = ZstdReader::new(zstd::Decoder::new(raw_stream)?, self.data_size);
                //reader.source.decode_to(Offset(self.data_size.0))?;
                Box::new(reader)
            }
            CompressionType::None => unreachable!(),
        };
        *cluster_reader = ClusterReader::Plain(decompress_reader);
        Ok(())
    }

    #[cfg(test)]
    fn blob_count(&self) -> BlobCount {
        BlobCount::from((self.blob_offsets.len() - 1) as u16)
    }

    pub fn get_reader(&self, index: BlobIdx) -> Result<Box<dyn Reader>> {
        self.build_plain_reader()?;
        let offset = self.blob_offsets[index.into_usize()];
        let end_offset = self.blob_offsets[index.into_usize() + 1];
        if let ClusterReader::Plain(r) = &*self.reader.borrow() {
            Ok(r.create_sub_reader(offset, End::Offset(end_offset)))
        } else {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use test_case::test_case;

    fn create_cluster(comp: CompressionType, data: &[u8]) -> (SizedOffset, Vec<u8>) {
        let mut cluster_data = Vec::new();
        cluster_data.extend_from_slice(&data);
        #[rustfmt::skip]
        cluster_data.extend_from_slice(&[
            comp as u8,       // compression
            0x01,             // offset_size
            0x00, 0x03,       // blob_count
            data.len() as u8, // raw data size
            0x0f,             // Data size
            0x05,             // Offset of blob 1
            0x08,             // Offset of blob 2
        ]);
        (
            SizedOffset::new(
                Size::from(cluster_data.len() - data.len()),
                Offset(data.len() as u64),
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

    fn create_lzma_cluster() -> (SizedOffset, Vec<u8>) {
        let indata = vec![
            0x11, 0x12, 0x13, 0x14, 0x15, // Blob 0
            0x21, 0x22, 0x23, // Blob 1
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Blob 3
        ];
        let data = {
            let compressed_content = Vec::new();
            let mut encoder =
                lzma::LzmaWriter::new_compressor(Cursor::new(compressed_content), 9).unwrap();
            let mut incursor = Cursor::new(indata);
            std::io::copy(&mut incursor, &mut encoder).unwrap();
            encoder.finish().unwrap().into_inner()
        };
        create_cluster(CompressionType::Lzma, &data)
    }

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

    #[test_case(CompressionType::None, create_raw_cluster)]
    #[test_case(CompressionType::Lz4, create_lz4_cluster)]
    #[test_case(CompressionType::Lzma, create_lzma_cluster)]
    #[test_case(CompressionType::Zstd, create_zstd_cluster)]
    fn test_cluster(comp: CompressionType, creator: ClusterCreator) {
        let (ptr_info, data) = creator();
        let reader = BufReader::new(data, End::None);
        let mut stream = reader.create_stream_from(ptr_info.offset);
        let header = ClusterHeader::produce(stream.as_mut()).unwrap();
        assert_eq!(header.compression, comp);
        assert_eq!(header.offset_size, 1);
        assert_eq!(header.blob_count, 3.into());
        let cluster = Cluster::new(&reader, ptr_info).unwrap();
        assert_eq!(cluster.blob_count(), 3.into());

        {
            let sub_reader = cluster.get_reader(BlobIdx::from(0)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(5_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x11, 0x12, 0x13, 0x14, 0x15]);
        }
        {
            let sub_reader = cluster.get_reader(BlobIdx::from(1)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(3_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x21, 0x22, 0x23]);
        }
        {
            let sub_reader = cluster.get_reader(BlobIdx::from(2)).unwrap();
            assert_eq!(sub_reader.size(), Size::from(7_u64));
            let mut v = Vec::<u8>::new();
            let mut stream = sub_reader.create_stream_all();
            stream.read_to_end(&mut v).unwrap();
            assert_eq!(v, [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]);
        }
    }
}
