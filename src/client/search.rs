use crate::{
    Client,
    models::{
        error::Error,
        query::{
            AlbumQuery, PlaylistsQuery, SearchAllQuery, SearchResultsQuery, TracksQuery, UsersQuery,
        },
        response::{Playlists, SearchAllResponse, SearchResultsResponse, Tracks, Users},
    },
};

impl Client {
    pub async fn search_all(
        &self,
        query: Option<&SearchAllQuery>,
    ) -> Result<SearchAllResponse, Error> {
        self.get_with_query("search", query).await
    }

    pub async fn search_tracks(&self, query: Option<&TracksQuery>) -> Result<Tracks, Error> {
        self.get_with_query("search/tracks", query).await
    }

    pub async fn search_playlists(
        &self,
        query: Option<&PlaylistsQuery>,
    ) -> Result<Playlists, Error> {
        self.get_with_query("search/playlists", query).await
    }

    pub async fn search_users(&self, query: Option<&UsersQuery>) -> Result<Users, Error> {
        self.get_with_query("search/users", query).await
    }

    pub async fn search_albums(&self, query: Option<&AlbumQuery>) -> Result<Playlists, Error> {
        self.get_with_query("search/albums", query).await
    }

    pub async fn get_search_results(
        &self,
        query: Option<&SearchResultsQuery>,
    ) -> Result<SearchResultsResponse, Error> {
        self.get_with_query("search/queries", query).await
    }
}
