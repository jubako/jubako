use crate::bases::*;
use log::trace;
use log::warn;
use reqwest::blocking::ClientBuilder;
use reqwest::{blocking::Client, IntoUrl, Url};
use std::borrow::Cow;
use std::io::ErrorKind;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

fn get_file_size(client: &Client, url: &str) -> std::io::Result<u64> {
    let response = client
        .head(url)
        .send()
        .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?
        .error_for_status()
        .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
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
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
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

        let response = response.map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(std::io::Error::new(
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

    fn read(&self, offset: Offset, mut buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut resp = self.fetch_at(offset.into_u64(), buf.len() as u64)?;
        resp.copy_to(&mut buf)
            .map(|r| r as usize)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
    }

    fn read_exact(&self, offset: Offset, mut buf: &mut [u8]) -> std::io::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let mut resp = self.fetch_at(offset.into_u64(), buf.len() as u64)?;
        resp.copy_to(&mut buf)
            .map(|_| ())
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))
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
        resp.copy_to(&mut buf)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
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
