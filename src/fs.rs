use std::path::Path;

use futures::{Stream, StreamExt};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
};

use crate::error::Error;

/// Dedicated file system service for soundcloud-rs managing directories, files, and stream writing.
pub struct FileService;

impl FileService {
    /// Ensure that a directory exists, creating all parent components if necessary.
    pub async fn ensure_dir(path: &Path) -> Result<(), Error> {
        fs::create_dir_all(path).await?;
        Ok(())
    }

    /// Ensure that the parent directory of a file path exists.
    pub async fn ensure_parent_dir(file_path: &Path) -> Result<(), Error> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// Safely remove a file from disk if it exists, ignoring `NotFound`.
    pub async fn remove_file_if_exists(path: &Path) -> Result<(), Error> {
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Read the file size in bytes.
    pub async fn file_size(path: &Path) -> Result<u64, Error> {
        let meta = fs::metadata(path).await?;
        Ok(meta.len())
    }

    /// Writes an incoming byte stream into a file on disk, calling `on_progress` for each chunk.
    pub async fn write_stream_to_file<S, B, E, F>(
        output_path: &Path,
        mut stream: S,
        total_size: Option<u64>,
        mut on_progress: F,
    ) -> Result<u64, Error>
    where
        S: Stream<Item = Result<B, E>> + Unpin,
        B: AsRef<[u8]>,
        E: Into<Error>,
        F: FnMut(u64, Option<u64>),
    {
        Self::ensure_parent_dir(output_path).await?;
        let mut writer = BufWriter::new(fs::File::create(output_path).await?);
        let mut downloaded: u64 = 0;

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(Into::into)?;
            let slice = chunk.as_ref();
            writer.write_all(slice).await?;
            downloaded += slice.len() as u64;
            on_progress(downloaded, total_size);
        }

        writer.flush().await?;
        Ok(downloaded)
    }
}
