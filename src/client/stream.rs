use crate::{
    models::{client::Client, error::Error, response::Track},
    response::{Stream, StreamType, Transcoding},
};

impl Client {
    /// Resolve stream URL and type from an already-fetched Track without re-fetching it.
    pub async fn resolve_stream_from_track(
        &self,
        track: &Track,
    ) -> Result<(String, StreamType), Error> {
        let transcodings = track
            .media
            .as_ref()
            .and_then(|m| m.transcodings.as_ref())
            .ok_or(Error::MissingTranscodings)?;

        let mut last_err = None;
        for t in transcodings {
            if t.snipped.unwrap_or(false) || t.preset.as_deref() == Some("abr_sq") {
                continue;
            }
            let Some(protocol) = t.format.as_ref().and_then(|f| f.protocol.clone()) else {
                continue;
            };

            match self.resolve_transcoding_url(t).await {
                Ok(url) => return Ok((url, protocol)),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or(Error::NoPlayableStream))
    }

    /// Resolve stream URL and type by track ID.
    pub async fn resolve_stream(&self, id: i64) -> Result<(String, StreamType), Error> {
        let track = self.get_track(id).await?;
        self.resolve_stream_from_track(&track).await
    }

    async fn resolve_transcoding_url(&self, transcoding: &Transcoding) -> Result<String, Error> {
        let path = transcoding
            .url
            .as_ref()
            .ok_or(Error::MissingTranscodingUrl)?;
        let (stream, _): (Stream, _) = self.get_json(path, None, None::<&()>).await?;
        stream.url.ok_or(Error::MissingStreamUrl)
    }
}
