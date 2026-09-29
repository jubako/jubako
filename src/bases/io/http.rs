use crate::bases::*;
use log::trace;
use log::warn;
use reqwest::blocking::ClientBuilder;
use reqwest::{blocking::Client, IntoUrl, Url};
use std::borrow::Cow;
use std::io::{self, ErrorKind, Read};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

fn to_io_error(err: reqwest::Error) -> io::Error {
    let kind = if err.is_timeout() {
        io::ErrorKind::TimedOut
    } else if err.is_connect() {
        io::ErrorKind::ConnectionRefused
    } else if let Some(status) = err.status() {
        match status.as_u16() {
            404 => io::ErrorKind::NotFound,
            401 | 403 => io::ErrorKind::PermissionDenied,
            408 => io::ErrorKind::TimedOut,
            409 => io::ErrorKind::AlreadyExists,
            400..=499 => io::ErrorKind::InvalidInput,
            500..=599 => io::ErrorKind::Other,
            _ => io::ErrorKind::Other,
        }
    } else if err.is_decode() {
        io::ErrorKind::InvalidData
    } else {
        io::ErrorKind::Other
    };

    io::Error::new(kind, err)
}

fn get_file_size(client: &Client, url: &str) -> std::io::Result<u64> {
    let response = client
        .head(url)
        .send()
        .map_err(to_io_error)?
        .error_for_status()
        .map_err(to_io_error)?;
    let size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .ok_or(std::io::Error::new(ErrorKind::Other, "No header"))
        .and_then(|v| {
            v.to_str()
                .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
        })
        .and_then(|s| {
            s.parse::<u64>()
                .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
        });

    size
}

pub struct HttpSource {
    client: Client,
    url: Url,
    len: u64,
    request_nb: AtomicU64,
}

impl HttpSource {
    pub fn client() -> std::io::Result<Client> {
        ClientBuilder::new()
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .user_agent(APP_USER_AGENT)
            .build()
            .map_err(to_io_error)
    }

    pub fn open(url: impl IntoUrl) -> std::io::Result<Self> {
        Self::new_with_client(url, Self::client()?)
    }

    pub fn new_with_client(url: impl IntoUrl, client: Client) -> std::io::Result<Self> {
        let url = url.into_url().unwrap();
        let size = get_file_size(&client, url.as_str())?;
        Ok(Self {
            url,
            len: size,
            client,
            request_nb: AtomicU64::new(0),
        })
    }

    fn fetch_at(&self, start: u64, len: u64) -> std::io::Result<reqwest::blocking::Response> {
        let end = start + len.saturating_sub(1);

        let request = self
            .client
            .get(self.url.clone())
            .header(reqwest::header::RANGE, format!("bytes={start}-{end}"));
        let request_nb = self
            .request_nb
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        trace!("Request #{request_nb}: {request:?}");
        let response = request.send();
        trace!("Response #{request_nb}: {response:?}");

        let response = response.map_err(to_io_error)?;

        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(io::Error::new(
                ErrorKind::Unsupported,
                "Range not supported",
            ));
        }

        Ok(response)
    }
}

#[cfg(target_pointer_width = "64")]
#[inline]
const fn move_to_memory(_region: Region) -> bool {
    true
}

#[cfg(target_pointer_width = "32")]
#[inline]
fn move_to_memory(region: Region) -> bool {
    let max_memory_block_size = option_env!("JBK_MAX_MEMORY_BLOC")
        .map(|s| {
            s.parse::<usize>()
                .expect(&format!("{s} should be a parsing size."))
        })
        .unwrap_or(0xFFFFFF);
    region.size() <= Size::new(max_memory_block_size as u64)
}

impl Source for HttpSource {
    fn size(&self) -> Size {
        (self.len).into()
    }

    fn read(self: Arc<Self>, region: Region) -> Result<Box<dyn ReadSized>> {
        let resp = self.fetch_at(region.begin().into_u64(), region.size().into_u64())?;
        Ok(Box::new(ReadResponse::new(resp, region)))
    }

    fn read_exact(&self, offset: Offset, mut buf: &mut [u8]) -> std::io::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let mut resp = self.fetch_at(offset.into_u64(), buf.len() as u64)?;
        resp.copy_to(&mut buf).map(|_| ()).map_err(to_io_error)
    }

    fn get_slice(&self, region: ARegion, block_check: BlockCheck) -> Result<Cow<'_, [u8]>> {
        let mut buf = vec![0; region.size().into_usize() + block_check.size()];
        self.read_exact(region.begin(), &mut buf)?;
        if let BlockCheck::Crc32 = block_check {
            assert_slice_crc(&buf)?;
        }
        buf.truncate(region.size().into_usize());
        Ok(Cow::Owned(buf))
    }

    fn cut(
        self: Arc<Self>,
        region: Region,
        block_check: BlockCheck,
        in_memory: bool,
    ) -> Result<(Arc<dyn Source>, Region)> {
        if !move_to_memory(region) || !in_memory {
            if let BlockCheck::Crc32 = block_check {
                warn!("Check of not memory block is not implemented");
            }
            return Ok((self, region));
        }

        // We know from previous test that region.size() is addressable.
        let full_size = ASize::new(region.size().into_u64() as usize + block_check.size());
        let mut buf = Vec::with_capacity(full_size.into_usize());
        let mut resp = self.fetch_at(region.begin().into_u64(), full_size.into_u64())?;
        resp.copy_to(&mut buf).map_err(to_io_error)?;
        if let BlockCheck::Crc32 = block_check {
            assert_slice_crc(&buf)?;
        }
        Ok((
            Arc::new(buf),
            Region::new_from_size(Offset::zero(), region.size()),
        ))
    }

    fn display(&self) -> String {
        format!("Remote File {}", self.url.as_str())
    }
}

struct ReadResponse {
    response: reqwest::blocking::Response,
    offset: Offset,
    region: Region,
}

impl ReadResponse {
    fn new(response: reqwest::blocking::Response, region: Region) -> Self {
        Self {
            response,
            offset: region.begin(),
            region,
        }
    }
}

impl ReadSized for ReadResponse {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.response.read(buf)?;
        self.offset += len;
        Ok(len)
    }
    fn size_left(&self) -> Size {
        self.region.end() - self.offset
    }

    fn size(&self) -> Size {
        self.region.size()
    }

    fn offset(&self) -> Offset {
        (self.offset - self.region.begin()).into_u64().into()
    }
}
