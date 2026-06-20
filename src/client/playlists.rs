use std::path::PathBuf;

use crate::models::{
    client::{Client, Identifier},
    error::Error,
    query::{Paging, PlaylistsQuery},
    response::{Playlist, Playlists, Users},
};

impl Client {
    pub async fn search_playlists(
        &self,
        query: Option<&PlaylistsQuery>,
    ) -> Result<Playlists, Error> {
        let resp: Playlists = self.get("search/playlists", query).await?;
        Ok(resp)
    }

    pub async fn get_playlist(&self, identifier: &Identifier) -> Result<Playlist, Error> {
        let url = format!("playlists/{identifier}");
        let resp: Playlist = self.get(&url, None::<&()>).await?;
        Ok(resp)
    }

    pub async fn get_playlist_reposters(
        &self,
        identifier: &Identifier,
        pagination: Option<&Paging>,
    ) -> Result<Users, Error> {
        let url = format!("playlists/{identifier}/reposters");
        let resp: Users = self.get(&url, pagination).await?;
        Ok(resp)
    }

    pub async fn download_playlist(
        &self,
        identifier: &Identifier,
        destination: Option<&str>,
        playlist_name: Option<&str>,
    ) -> Result<(), Error> {
        let playlist = self.get_playlist(identifier).await?;

        let playlist_title = match playlist_name {
            Some(playlist_name) => playlist_name,
            None => playlist
                .title
                .as_deref()
                .ok_or_else(|| Error::new("Missing playlist title"))?,
        };

        let output_path = match destination {
            Some(destination) => PathBuf::from(destination).join(playlist_title),
            None => PathBuf::from(playlist_title),
        };
        if !output_path.exists() {
            std::fs::create_dir_all(&output_path)?;
        }

        let output_path_str = output_path
            .to_str()
            .ok_or_else(|| Error::new("Output path contains invalid UTF-8"))?;

        let tracks = playlist
            .tracks
            .as_ref()
            .ok_or_else(|| Error::new("Missing tracks in playlist"))?;
        for track in tracks {
            let identifier = track
                .id
                .ok_or_else(|| Error::new("Missing track id in playlist"))?;

            if let Err(e) = self
                .download_track(
                    &track,
                    &Identifier::Id(identifier),
                    None,
                    Some(output_path_str),
                    None,
                )
                .await
            {
                println!("Error downloading track: {e}")
            }
        }

        Ok(())
    }
}
