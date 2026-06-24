use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use ffmpeg_sidecar::{command::FfmpegCommand, download};

use crate::{
    models::{
        client::{Client, Identifier},
        error::Error,
        query::{Paging, TracksQuery},
        response::{Track, Tracks},
    },
    response::{Stream, StreamType, Transcoding, Waveform},
};

static FFMPEG_READY: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_ffmpeg() -> Result<(), Error> {
    if ffmpeg_sidecar::command::ffmpeg_is_installed() {
        return Ok(());
    }
    let result = FFMPEG_READY.get_or_init(|| download::auto_download().map_err(|e| format!("{e}")));
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::new(format!("FFmpeg not available: {e}"))),
    }
}

impl Client {
    pub async fn search_tracks(&self, query: Option<&TracksQuery>) -> Result<Tracks, Error> {
        let tracks: Tracks = self.get("search/tracks", query).await?;
        Ok(tracks)
    }

    pub async fn get_track(&self, identifier: &Identifier) -> Result<Track, Error> {
        let url = format!("tracks/{identifier}");
        let resp: Track = self.get(&url, None::<&()>).await?;
        Ok(resp)
    }

    pub async fn get_track_related(
        &self,
        identifier: &Identifier,
        pagination: Option<&Paging>,
    ) -> Result<Tracks, Error> {
        let url = format!("tracks/{identifier}/related");
        let resp: Tracks = self.get(&url, pagination).await?;
        Ok(resp)
    }

    pub async fn download_track(
        &self,
        track: &Track,
        _identifier: &Identifier,
        stream_type: Option<&StreamType>,
        destination: Option<&str>,
        filename: Option<&str>,
    ) -> Result<(), Error> {
        let stream = stream_type.unwrap_or(&StreamType::Progressive);

        let title = match filename {
            Some(filename) => filename,
            None => track
                .title
                .as_deref()
                .ok_or_else(|| Error::new("Track title is missing"))?,
        };

        let output_path = match destination {
            Some(destination) => PathBuf::from(destination).join(format!("{title}.mp3")),
            None => PathBuf::from(format!("{title}.mp3")),
        };
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let transcoding = self.get_transcoding_by_stream_type(track, stream).await?;
        let stream_url = self.resolve_transcoding_url(&transcoding).await?;

        let protocol = transcoding
            .format
            .as_ref()
            .and_then(|f| f.protocol.as_ref())
            .ok_or_else(|| Error::new("Missing transcoding format/protocol"))?;

        match protocol {
            StreamType::Progressive => self.download_progressive(&stream_url, &output_path).await?,
            StreamType::Hls => self.download_hls_to_file(&stream_url, &output_path).await?,
            _ => return Err(Error::new("Invalid Stream Type")),
        }

        Ok(())
    }

    pub async fn get_track_waveform(&self, identifier: &Identifier) -> Result<Waveform, Error> {
        let track = self.get_track(identifier).await?;
        let waveform_url = track
            .waveform_url
            .as_ref()
            .ok_or_else(|| Error::new("Missing waveform URL"))?;
        let response = self.http_client.get(waveform_url).send().await?;
        let waveform: Waveform = response.json::<Waveform>().await?;
        Ok(waveform)
    }

    /// Resolve stream URL from an already-fetched Track without re-fetching it.
    pub async fn resolve_stream_url_from_track(
        &self,
        track: &Track,
        stream_type: Option<&StreamType>,
    ) -> Result<String, Error> {
        let stream = stream_type.unwrap_or(&StreamType::Progressive);
        let transcoding = self.get_transcoding_by_stream_type(track, stream).await?;
        self.resolve_transcoding_url(&transcoding).await
    }

    /// Resolve stream URL by fetching the track first. Prefer `resolve_stream_url_from_track`
    /// if you already have the Track object to avoid a redundant HTTP request.
    pub async fn get_stream_url(
        &self,
        identifier: &Identifier,
        stream_type: Option<&StreamType>,
    ) -> Result<String, Error> {
        let track = self.get_track(identifier).await?;
        self.resolve_stream_url_from_track(&track, stream_type)
            .await
    }

    async fn resolve_transcoding_url(&self, transcoding: &Transcoding) -> Result<String, Error> {
        let path = transcoding
            .url
            .as_ref()
            .ok_or_else(|| Error::new("Missing transcoding URL"))?;
        let client_id = self.get_client_id_value().await;
        let (stream, _): (Stream, _) = self.get_json(path, None, None::<&()>, &client_id).await?;
        stream
            .url
            .ok_or_else(|| Error::new("Missing resolved stream URL"))
    }

    async fn get_transcoding_by_stream_type(
        &self,
        track: &Track,
        stream_type: &StreamType,
    ) -> Result<Transcoding, Error> {
        let transcodings = track
            .media
            .as_ref()
            .and_then(|m| m.transcodings.as_ref())
            .ok_or_else(|| Error::new("Missing media transcodings"))?;

        let mut filtered = transcodings.iter().filter(|t| {
            t.format
                .as_ref()
                .and_then(|f| f.protocol.as_ref())
                .map_or(false, |p| p == stream_type)
        });

        if let Some(non_snipped) = filtered.clone().find(|t| !t.snipped.unwrap_or(false)) {
            return Ok(non_snipped.clone());
        }

        if filtered.any(|t| t.snipped.unwrap_or(false)) {
            Err(Error::new(
                "Track is a premium Go+ track (only preview snippet is available)",
            ))
        } else {
            Err(Error::new("No available download options"))
        }
    }

    async fn download_progressive(
        &self,
        stream_url: &str,
        output_path: &Path,
    ) -> Result<(), Error> {
        let response = self.http_client.get(stream_url).send().await?;
        let bytes = response.bytes().await?;
        tokio::fs::write(output_path, &bytes).await?;
        Ok(())
    }

    /// Download an HLS stream to an MP3 file using FFmpeg.
    /// `stream_url` should be an already-resolved HLS playlist URL.
    pub async fn download_hls_to_file(
        &self,
        stream_url: &str,
        output_path: &Path,
    ) -> Result<(), Error> {
        let stream_url = stream_url.to_string();
        let output_path = output_path.to_path_buf();
        let proxy_url = self.proxy_url.clone();

        tokio::task::spawn_blocking(move || {
            ensure_ffmpeg()?;
            let output_str = output_path
                .to_str()
                .ok_or_else(|| Error::new("Output path contains invalid UTF-8"))?;
            let mut ffmpeg = FfmpegCommand::new();
            if let Some(ref proxy) = proxy_url {
                ffmpeg.arg("-http_proxy").arg(proxy);
            }
            let status = ffmpeg
                .input(&stream_url)
                .output(output_str)
                .args(["-codec:a", "libmp3lame", "-q:a", "2"])
                .spawn()
                .map_err(|e| Error::new(format!("FFmpeg spawn failed: {e}")))?
                .wait()
                .map_err(|e| Error::new(format!("FFmpeg wait failed: {e}")))?;

            if !status.success() {
                return Err(Error::new("HLS download failed"));
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::new(format!("Tokio spawn_blocking failed: {e}")))??;

        Ok(())
    }

    pub async fn get_tracks(&self, ids: &[i64]) -> Result<Vec<Track>, Error> {
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        #[derive(serde::Serialize)]
        struct GetTracksQuery {
            ids: String,
        }

        self.get("tracks", Some(&GetTracksQuery { ids: ids_str }))
            .await
    }
}
