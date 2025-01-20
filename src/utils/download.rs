use std::cmp::min;
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use indicatif::ProgressBar;
use reqwest::{header, Client};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;

use super::progress_bar::{CliProgress, Style};
use crate::core::GlobalOpts;
use crate::setter;
use crate::toolset_manifest::Proxy as CrateProxy;

fn default_proxy() -> reqwest::Proxy {
    reqwest::Proxy::custom(|url| env_proxy::for_url(url).to_url())
        .no_proxy(reqwest::NoProxy::from_env())
}

#[derive(Debug)]
pub struct DownloadOpt<T: Sized> {
    /// The verbose name of the file to download.
    pub name: String,
    /// Download progress handler, aka a progress bar.
    pub handler: Option<CliProgress<T>>,
    /// Option to skip SSL certificate verification when downloading.
    pub insecure: bool,
    /// Proxy configurations for download.
    pub proxy: Option<CrateProxy>,
    /// Whether or not to resuming previous download.
    resume: bool,
}

impl DownloadOpt<ProgressBar> {
    pub fn new<S: ToString>(name: S) -> Self {
        let handler = (!GlobalOpts::get().quiet).then_some(CliProgress::new());
        Self {
            name: name.to_string(),
            handler,
            insecure: false,
            proxy: None,
            resume: false,
        }
    }

    setter!(with_proxy(self.proxy, Option<CrateProxy>));
    setter!(with_handler(self.handler, Option<CliProgress<ProgressBar>>));
    setter!(insecure(self.insecure, bool));
    setter!(resume(self.resume, bool));

    /// Build and return a client for download
    fn client(&self) -> Result<Client> {
        let user_agent = format!("{}/{}", t!("vendor_en"), env!("CARGO_PKG_VERSION"));
        let proxy = if let Some(p) = &self.proxy {
            p.try_into()?
        } else {
            default_proxy()
        };
        let client = Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(self.insecure)
            .proxy(proxy)
            .build()?;
        Ok(client)
    }

    /// Consume self, and retrive text response by sending request to a given url.
    ///
    /// If the `url` is a local file, this will use [`read_to_string`](fs::read_to_string) to
    /// get the text instead.
    pub async fn read(self, url: &Url) -> Result<String> {
        if url.scheme() == "file" {
            let file_url = url
                .to_file_path()
                .map_err(|_| anyhow!("file url does not exist"))?;
            return fs::read_to_string(&file_url).await.with_context(|| {
                format!(
                    "unable to read {} located in {}",
                    self.name,
                    file_url.display()
                )
            });
        }

        if self.insecure {
            warn!("{}", t!("insecure_download"));
        }

        let resp = self
            .client()?
            .get(url.as_ref())
            .send()
            .await
            .with_context(|| format!("failed to receive surver response from '{url}'"))?;
        if resp.status().is_success() {
            Ok(resp.text().await?)
        } else {
            bail!(
                "unable to get text content of url '{url}': server responded with error {}",
                resp.status()
            );
        }
    }
    /// Consume self, and download from given `Url` to `Path`.
    pub async fn download(self, url: &Url, path: &Path) -> Result<()> {
        if url.scheme() == "file" {
            fs::copy(
                url.to_file_path()
                    .map_err(|_| anyhow!("unable to convert to file path for url '{url}'"))?,
                path,
            )
            .await?;
            return Ok(());
        }

        if self.insecure {
            warn!("{}", t!("insecure_download"));
        }

        let helper = DownloadHelper::new(&self.client()?, url, path, self.resume).await?;
        let (mut resp, mut file, mut downloaded_bytes) =
            (helper.response, helper.file, helper.downloaded_bytes);

        let total_size = resp
            .content_length()
            .ok_or_else(|| anyhow!("unable to get file length of '{url}'"))?;

        let maybe_indicator = self.handler.as_ref().and_then(|h| {
            (h.start)(
                format!("downloading '{}'", &self.name),
                Style::Bytes(total_size),
            )
            .ok()
        });

        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;

            downloaded_bytes = min(downloaded_bytes + chunk.len() as u64, total_size);
            if let Some(indicator) = &maybe_indicator {
                // safe to unwrap, because indicator won't exist if self.handler is none
                (self.handler.as_ref().unwrap().update)(indicator, Some(downloaded_bytes));
            }
        }

        if let Some(indicator) = &maybe_indicator {
            // safe to unwrap, because indicator won't exist if self.handler is none
            (self.handler.as_ref().unwrap().stop)(
                indicator,
                format!("'{}' successfully downloaded.", &self.name),
            );
        }

        Ok(())
    }

    /// Consume self, and download from given `Url` to `Path`.
    ///
    /// Note: This will block the current thread until the download is finished.
    pub fn blocking_download(self, url: &Url, path: &Path) -> Result<()> {
        super::blocking!(self.download(url, path))
    }
}

struct DownloadHelper {
    response: reqwest::Response,
    file: fs::File,
    /// The length of bytes that already got downloaded.
    downloaded_bytes: u64,
}

impl DownloadHelper {
    async fn new_without_resume(client: &Client, url: &Url, path: &Path) -> Result<Self> {
        let response = get_response_(client, url, None).await?;
        let file = open_file_(path, true).await?;

        Ok(Self {
            response,
            file,
            downloaded_bytes: 0,
        })
    }

    async fn new(client: &Client, url: &Url, path: &Path, resume: bool) -> Result<Self> {
        let (downloaded_bytes, file) = if resume {
            let file = open_file_(path, false).await?;
            let downloaded = file.metadata().await?.len();
            (downloaded, file)
        } else {
            (0, open_file_(path, true).await?)
        };

        // resume from the next of downloaded byte
        let resume_from = (downloaded_bytes != 0).then_some(downloaded_bytes + 1);
        let response = get_response_(client, url, resume_from).await?;

        let status = response.status();
        if status == 416 {
            // 416: server does not support download range, retry without resuming
            info!("download range not satisfiable, retrying without ranges header");

            return Self::new_without_resume(client, url, path).await;
        } else if !status.is_success() {
            bail!("server returns error when attempting download from '{url}': {status}");
        }

        Ok(Self {
            response,
            file,
            downloaded_bytes,
        })
    }
}

async fn open_file_(path: &Path, truncate: bool) -> Result<fs::File> {
    Ok(fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(truncate)
        .open(path)
        .await?)
}

async fn get_response_(
    client: &Client,
    url: &Url,
    resume_from: Option<u64>,
) -> Result<reqwest::Response> {
    let mut builder = client.get(url.as_ref());
    if let Some(bytes) = resume_from {
        builder = builder.header(header::RANGE, format!("bytes={bytes}-"));
    }
    let resp = builder.send().await.with_context(|| {
        format!("failed to receive surver response when downloading from '{url}'")
    })?;
    Ok(resp)
}
