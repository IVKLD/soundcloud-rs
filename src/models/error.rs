use thiserror::Error;

/// Error type for SoundCloud API operations
#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Task join failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("URL parsing failed: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("Client ID not found")]
    ClientIdNotFound,

    #[error("No playable stream found")]
    NoPlayableStream,

    #[error("Missing waveform URL")]
    MissingWaveformUrl,

    #[error("Missing media transcodings")]
    MissingTranscodings,

    #[error("Missing transcoding URL")]
    MissingTranscodingUrl,

    #[error("Missing resolved stream URL")]
    MissingStreamUrl,

    #[error("Missing playlist title")]
    MissingPlaylistTitle,

    #[error("Missing tracks in playlist")]
    MissingPlaylistTracks,

    #[error("Missing user ID")]
    MissingUserId,

    #[error("Could not extract ID from URN")]
    InvalidUrn,

    #[error("Output path contains invalid UTF-8")]
    InvalidPath,

    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),

    #[error("HLS download failed")]
    HlsDownloadFailed,

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    pub fn with_status(status: u16, msg: impl Into<String>) -> Self {
        Self::HttpStatus {
            status,
            message: msg.into(),
        }
    }

    pub fn is_status(&self, code: u16) -> bool {
        match self {
            Self::HttpStatus { status, .. } => *status == code,
            Self::Http(err) => err.status().map_or(false, |s| s.as_u16() == code),
            _ => false,
        }
    }

    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            Self::Http(err) => err.status().map(|s| s.as_u16()),
            _ => None,
        }
    }
}
