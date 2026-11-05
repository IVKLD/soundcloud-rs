use crate::{
    models::{
        client::{Client, Identifier},
        error::Error,
        query::Paging,
        response::{Track, Tracks},
    },
    response::Waveform,
};

impl Client {
    pub async fn get_track(&self, identifier: impl Into<Identifier>) -> Result<Track, Error> {
        self.get(&format!("tracks/{}", identifier.into())).await
    }

    pub async fn get_tracks(&self, ids: &[i64]) -> Result<Vec<Track>, Error> {
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        self.get_with_query("tracks", Some(&[("ids", ids_str)]))
            .await
    }

    pub async fn get_track_related(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Tracks, Error> {
        self.get_with_query(&format!("tracks/{}/related", identifier.into()), pagination)
            .await
    }

    pub async fn get_track_waveform(
        &self,
        identifier: impl Into<Identifier>,
    ) -> Result<Waveform, Error> {
        let track = self.get_track(identifier).await?;
        let waveform_url = track
            .waveform_url
            .as_ref()
            .ok_or(Error::MissingWaveformUrl)?;
        self.ensure_proxy_refreshed().await?;
        let http = self.http_client.read().await;
        let response = http.get(waveform_url).send().await?.error_for_status()?;
        let waveform: Waveform = response.json::<Waveform>().await?;
        Ok(waveform)
    }
}
