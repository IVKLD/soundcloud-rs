use crate::models::{
    client::{Client, Identifier},
    error::Error,
    query::{Paging, UserTrackLikesQuery},
    response::{Playlists, Reposts, Track, Tracks, User, UserTrackLikes, Users},
};

impl Client {
    pub async fn get_user(&self, identifier: impl Into<Identifier>) -> Result<User, Error> {
        self.get(&format!("users/{}", identifier.into())).await
    }

    pub async fn get_user_followers(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Users, Error> {
        self.get_with_query(
            &format!("users/{}/followers", identifier.into()),
            pagination,
        )
        .await
    }

    pub async fn get_user_followings(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Users, Error> {
        self.get_with_query(
            &format!("users/{}/followings", identifier.into()),
            pagination,
        )
        .await
    }

    pub async fn get_user_playlists(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Playlists, Error> {
        self.get_with_query(
            &format!("users/{}/playlists", identifier.into()),
            pagination,
        )
        .await
    }

    pub async fn get_user_tracks(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Tracks, Error> {
        self.get_with_query(&format!("users/{}/tracks", identifier.into()), pagination)
            .await
    }

    pub async fn get_user_reposts(
        &self,
        identifier: impl Into<Identifier>,
        pagination: Option<&Paging>,
    ) -> Result<Reposts, Error> {
        let identifier = identifier.into();
        let id = match &identifier {
            Identifier::Id(id) => id.to_string(),
            Identifier::Urn(urn) => urn.split(':').last().ok_or(Error::InvalidUrn)?.to_owned(),
        };
        self.get_with_query(&format!("stream/users/{id}/reposts"), pagination)
            .await
    }

    /// Fetches all liked tracks of a user, automatically following pagination links.
    pub async fn get_user_likes(
        &self,
        identifier: impl Into<Identifier>,
    ) -> Result<Vec<Track>, Error> {
        let identifier = identifier.into();
        let mut offset: Option<String> = None;
        let mut tracks = Vec::new();

        loop {
            let query = UserTrackLikesQuery {
                limit: Some(50),
                offset: offset.clone(),
            };
            let res: UserTrackLikes = self
                .get_with_query(&format!("users/{identifier}/track_likes"), Some(&query))
                .await?;
            if res.collection.is_empty() {
                break;
            }

            tracks.extend(res.collection.into_iter().filter_map(|item| item.track));

            let Some(href) = res.next_href else {
                break;
            };
            match reqwest::Url::parse(&href)?
                .query_pairs()
                .find(|(k, _)| k == "offset")
                .map(|(_, v)| v.into_owned())
            {
                Some(next_offset) => offset = Some(next_offset),
                None => break,
            }
        }

        Ok(tracks)
    }
}
