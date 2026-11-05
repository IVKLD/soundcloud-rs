use crate::models::{
    client::{Client, Identifier},
    error::Error,
    query::Paging,
    response::{Playlist, Track, Users},
};

impl Client {
    pub async fn get_playlist(&self, identifier: impl Into<Identifier>) -> Result<Playlist, Error> {
        self.get(&format!("playlists/{}", identifier.into())).await
    }

    /// Fetches all full tracks for a playlist by identifier, resolving any stub/incomplete tracks via batch requests.
    pub async fn get_playlist_tracks(
        &self,
        identifier: impl Into<Identifier>,
    ) -> Result<Vec<Track>, Error> {
        let playlist = self.get_playlist(identifier).await?;
        self.resolve_playlist_tracks(&playlist).await
    }

    pub async fn get_playlist_reposters(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Users, Error> {
        self.get_with_query(
            &format!("playlists/{}/reposters", identifier.into()),
            pagination,
        )
        .await
    }
}
