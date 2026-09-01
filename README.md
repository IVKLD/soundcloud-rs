<p align="center">
  <h2 align="center">soundcloud-rs</h2>
</p>

<p align="center">
  Asynchronous, type-safe Rust client for the SoundCloud API with automatic Client ID discovery, retry resilience, and streaming support.
  <br>
  <img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/Rust-2024_Edition-orange?style=flat-square&logo=rust" alt="Rust Edition">
  <img src="https://img.shields.io/badge/SoundCloud-v2_API-ff5500?style=flat-square&logo=soundcloud" alt="SoundCloud">
</p>

---

## Overview

`soundcloud-rs` provides an idiomatic, non-blocking Rust interface to search, resolve, stream, and download audio tracks from SoundCloud. It automatically discovers and refreshes valid client IDs from public web assets, eliminating the need for manual API key management.

## Features

- **Dynamic Client ID Discovery**: Automatically extracts client IDs and refreshes them upon `401 Unauthorized` responses.
- **Search & Discovery**: Support for tracks, playlists, albums, artists, reposts, and related tracks.
- **Stream Resolution**: Resolves direct high-quality audio streams with fallback support for progressive MP3 and HLS streams.
- **Proxy Support**: Configurable HTTP, HTTPS, and SOCKS5 proxy support.
- **Async & Non-Blocking**: Built on `tokio` and `reqwest` with connection pooling and configurable retry policies.

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
soundcloud-rs = { path = "libs/soundcloud-rs" }
tokio = { version = "1.53", features = ["full"] }
```

---

## Usage

### 1. Basic Search and Resolution

```rust
use soundcloud_rs::{Client, query::TracksQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new().await?;

    let query = TracksQuery {
        q: Some("Synthwave".into()),
        limit: Some(5),
        ..Default::default()
    };

    let tracks = client.search_tracks(Some(&query)).await?;
    for track in tracks.collection {
        let title = track.title.unwrap_or_default();
        let user = track.user.and_then(|u| u.username).unwrap_or_default();
        println!("Track: {} by {}", title, user);
    }

    Ok(())
}
```

### 2. ClientBuilder with Proxy and Custom Retries

```rust
use soundcloud_rs::{ClientBuilder, Identifier, query::TracksQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .with_proxy("http://127.0.0.1:8080")
        .with_max_retries(3)
        .with_retry_on_401(true)
        .build()
        .await?;

    let track = client.get_track(&Identifier::Id(123456789)).await?;
    println!("Found track: {:?}", track.title);

    Ok(())
}
```

### 3. Playlists and Profiles

```rust
use soundcloud_rs::{Client, Identifier, query::PlaylistsQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new().await?;

    let query = PlaylistsQuery {
        q: Some("Chillout".into()),
        limit: Some(3),
        ..Default::default()
    };

    let playlists = client.search_playlists(Some(&query)).await?;
    for playlist in playlists.collection {
        println!("Playlist: {}", playlist.title.unwrap_or_default());
    }

    Ok(())
}
```

---

## API Reference

### Client Operations
- `Client::new() -> Result<Self, Error>`: Initialize with auto-discovered `client_id`.
- `ClientBuilder::new() -> Self`: Builder for custom proxy, cached ID, and retry policies.
- `health_check(&self) -> bool`: Verify connectivity against `/me`.
- `refresh_client_id(&self) -> Result<(), Error>`: Force client ID redetection.

### Resources
- **Tracks**: `search_tracks`, `get_track`, `get_track_related`, `get_track_waveform`
- **Playlists**: `search_playlists`, `get_playlist`, `get_playlist_reposters`
- **Users**: `search_users`, `get_user`, `get_user_tracks`, `get_user_playlists`, `get_user_followers`

---

## License

MIT License.
