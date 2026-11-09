//! Files API: image storage for vision inputs, not a document/vector database.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileObject {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u64,
    pub filename: String,
    pub purpose: String,
    pub expires_at: Option<u64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListFilesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<FileOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOrder {
    Asc,
    Desc,
}
impl ListFilesRequest {
    pub fn validate(&self) -> Result<()> {
        if self.limit.is_some_and(|n| !(1..=1000).contains(&n))
            || self.purpose.as_deref().is_some_and(|s| s != "user_data")
        {
            return Err(Error::Configuration(
                "file limit must be 1..=1000 and purpose must be user_data".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileList {
    pub object: String,
    pub data: Vec<FileObject>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDeletion {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
/// In-memory image upload. Bytes are never logged. Expiry is optional (otherwise permanent).
pub struct FileUpload {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub expires_after_seconds: Option<u32>,
}
impl FileUpload {
    pub fn new(filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            bytes,
            expires_after_seconds: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.bytes.is_empty() || self.bytes.len() > 64 * 1024 * 1024 {
            return Err(Error::Configuration(
                "image upload must contain 1 byte to 64 MiB".into(),
            ));
        }
        if self
            .expires_after_seconds
            .is_some_and(|n| !(3600..=2592000).contains(&n))
        {
            return Err(Error::Configuration(
                "file expiry must be 3600..=2592000 seconds".into(),
            ));
        }
        Ok(())
    }
}
pub(crate) fn file_path(id: &str) -> Result<String> {
    if !id.starts_with("file-api-")
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::Configuration("invalid file ID".into()));
    }
    Ok(format!("files/{id}"))
}
