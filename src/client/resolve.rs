use std::collections::HashMap;

use crate::models::{
    client::Client,
    error::Error,
    response::{Playlist, ResolvedResource, Track, User},
};

impl Client {
    pub async fn resolve_url(&self, url: impl AsRef<str>) -> Result<ResolvedResource, Error> {
        let url_obj = reqwest::Url::parse(url.as_ref())?;
        let mut target_url = url.as_ref().to_string();
        if url_obj.host_str() == Some("on.soundcloud.com") {
            self.ensure_proxy_refreshed().await?;
            let http = self.http_client.read().await;
            let request = http.get(&target_url).send().await?;
            if !request.status().is_success() {
                return Err(Error::HttpStatus {
                    status: request.status().as_u16(),
                    message: format!("Failed to resolve on.soundcloud.com URL: {}", target_url),
                });
            }
            target_url = request.url().to_string();
        }
        self.get_with_query("resolve", Some(&[("url", target_url)]))
            .await
    }

    /// Resolves any SoundCloud URL (track, playlist, user likes, system playlist) directly into a list of tracks.
    pub async fn resolve_to_tracks(&self, url: impl AsRef<str>) -> Result<Vec<Track>, Error> {
        match self.resolve_url(url).await? {
            ResolvedResource::Track(track) => Ok(vec![track]),
            ResolvedResource::Playlist(playlist) => self.resolve_playlist_tracks(&playlist).await,
            ResolvedResource::User(user) => self.resolve_user_likes(&user).await,
            ResolvedResource::SystemPlaylist(sp) => Ok(sp.tracks.unwrap_or_default()),
        }
    }

    /// Resolves all tracks for an already-resolved playlist, batch-fetching any stub/incomplete tracks.
    pub async fn resolve_playlist_tracks(&self, playlist: &Playlist) -> Result<Vec<Track>, Error> {
        let collection = playlist
            .tracks
            .as_ref()
            .ok_or(Error::MissingPlaylistTracks)?;

        let mut tracks: Vec<Track> = collection.clone();

        let missing_ids: Vec<i64> = tracks
            .iter()
            .filter(|t| {
                t.title.as_deref().unwrap_or("").is_empty()
                    || t.title.as_deref() == Some("Unknown")
                    || t.media.is_none()
            })
            .filter_map(|t| t.id)
            .collect();

        if !missing_ids.is_empty() {
            let index: HashMap<i64, usize> = tracks
                .iter()
                .enumerate()
                .filter_map(|(i, t)| t.id.map(|id| (id, i)))
                .collect();

            for chunk in missing_ids.chunks(50) {
                if let Ok(fetched) = self.get_tracks(chunk).await {
                    for track in fetched {
                        if let Some(track_id) = track.id {
                            if let Some(&idx) = index.get(&track_id) {
                                tracks[idx] = track;
                            }
                        }
                    }
                }
            }
        }

        Ok(tracks)
    }

    /// Resolves all liked tracks for an already-resolved user.
    pub async fn resolve_user_likes(&self, user: &User) -> Result<Vec<Track>, Error> {
        let id = user.id.ok_or(Error::MissingUserId)?;
        self.get_user_likes(id).await
    }
}
