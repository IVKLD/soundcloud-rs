mod playlists;
mod reposts;
mod search;
mod tracks;
mod users;
pub use playlists::*;
pub use reposts::*;
pub use search::*;
use serde::{Deserialize, Serialize};
pub use tracks::*;
pub use users::*;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PagingCollection<T> {
    pub collection: Vec<T>,
}
