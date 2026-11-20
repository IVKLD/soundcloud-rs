use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

use ffmpeg_sidecar::{command::FfmpegCommand, download};

use crate::{
    fs::FileService,
    models::{client::Client, error::Error, response::Track},
    response::{StreamSource, StreamType},
};

pub type ProgressCallback = Arc<dyn Fn(u64, Option<u64>) + Send + Sync>;

static FFMPEG_READY: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_ffmpeg() -> Result<(), Error> {
    if ffmpeg_sidecar::command::ffmpeg_is_installed() {
        return Ok(());
    }
    let result = FFMPEG_READY.get_or_init(|| download::auto_download().map_err(|e| format!("{e}")));
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::Ffmpeg(e.clone())),
    }
}

impl Client {
    /// Unified stream downloader for both Progressive and HLS streams.
    pub async fn download_stream(
        &self,
        source: &StreamSource,
        output_path: &Path,
        on_progress: Option<&ProgressCallback>,
    ) -> Result<(), Error> {
        match source.stream_type {
            StreamType::Hls => {
                self.download_hls(&source.url, output_path, source.proxy_url.as_deref())
                    .await
            }
            _ => {
                self.download_progressive(&source.url, output_path, on_progress)
                    .await
            }
        }
    }

    /// Convenience helper to resolve and download a track directly to an output path.
    pub async fn download_track(
        &self,
        track: &Track,
        output_path: &Path,
        on_progress: Option<&ProgressCallback>,
    ) -> Result<(), Error> {
        FileService::ensure_parent_dir(output_path).await?;

        let (stream_url, stream_type) = self.resolve_stream_from_track(track).await?;
        let source = StreamSource {
            url: stream_url,
            stream_type,
            proxy_url: self.current_proxy_url().await,
        };
        self.download_stream(&source, output_path, on_progress)
            .await
    }

    async fn download_progressive(
        &self,
        stream_url: &str,
        output_path: &Path,
        on_progress: Option<&ProgressCallback>,
    ) -> Result<(), Error> {
        self.ensure_proxy_refreshed().await?;
        let http = self.http_client.read().await.clone();
        let response = http.get(stream_url).send().await?.error_for_status()?;
        let exact_size = response.content_length();
        let stream = response.bytes_stream();

        FileService::write_stream_to_file(
            output_path,
            stream,
            exact_size,
            move |downloaded, total| {
                if let Some(cb) = on_progress {
                    cb(downloaded, total);
                }
            },
        )
        .await?;

        Ok(())
    }

    /// Download an HLS stream to an MP3 file using FFmpeg.
    /// `stream_url` should be an already-resolved HLS playlist URL.
    async fn download_hls(
        &self,
        stream_url: &str,
        output_path: &Path,
        proxy_url: Option<&str>,
    ) -> Result<(), Error> {
        let stream_url = stream_url.to_string();
        let output_path = output_path.to_path_buf();
        self.ensure_proxy_refreshed().await?;
        let resolved_proxy = match proxy_url {
            Some(p) => Some(p.to_string()),
            None => self.current_proxy_url().await,
        };

        tokio::task::spawn_blocking(move || {
            ensure_ffmpeg()?;
            let output_str = output_path.to_str().ok_or(Error::InvalidPath)?;
            let mut ffmpeg = FfmpegCommand::new();
            if let Some(ref proxy) = resolved_proxy {
                ffmpeg.arg("-http_proxy").arg(proxy);
            }
            ffmpeg.overwrite();
            ffmpeg.args(["-rw_timeout", "15000000"]);
            ffmpeg
                .input(&stream_url)
                .args(["-codec:a", "libmp3lame", "-q:a", "2", "-f", "mp3"])
                .output(output_str)
                .spawn()?
                .wait()?
                .success()
                .then_some(())
                .ok_or(Error::HlsDownloadFailed)
        })
        .await?
    }
}
