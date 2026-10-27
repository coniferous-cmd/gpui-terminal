use std::{
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Context, bail, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const REVISION: u32 = 1;
pub const MAX_FRAME: usize = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub revision: u32,
    pub operation: Operation,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    List {},
    Create {
        window: String,
        name: Option<String>,
        cwd: Option<NativePath>,
    },
    Rename {
        id: String,
        name: String,
    },
    Focus {
        id: String,
    },
    Close {
        id: String,
        force: bool,
    },
}

impl Request {
    pub fn new(operation: Operation) -> Self {
        Self {
            revision: REVISION,
            operation,
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.revision == REVISION,
            "cli: unsupported protocol revision {}",
            self.revision
        );
        match &self.operation {
            Operation::Create { window, cwd, .. } => {
                validate_id(window)?;
                if let Some(cwd) = cwd {
                    ensure!(
                        cwd.to_path()?.is_absolute(),
                        "cli: working directory must be absolute"
                    );
                }
            }
            Operation::Rename { id, .. }
            | Operation::Focus { id }
            | Operation::Close { id, .. } => validate_id(id)?,
            Operation::List {} => {}
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> anyhow::Result<()> {
    ensure!(
        !id.is_empty() && id.len() <= 128,
        "cli: invalid runtime ID length"
    );
    ensure!(
        id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "cli: invalid runtime ID characters"
    );
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "platform",
    content = "units",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NativePath {
    Unix(Vec<u8>),
    Windows(Vec<u16>),
}

impl NativePath {
    pub fn from_path(path: &std::path::Path) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Self::Unix(path.as_os_str().as_bytes().to_vec())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Self::Windows(path.as_os_str().encode_wide().collect())
        }
    }

    pub fn to_path(&self) -> anyhow::Result<PathBuf> {
        match self {
            #[cfg(unix)]
            Self::Unix(bytes) => {
                use std::os::unix::ffi::OsStringExt;
                ensure!(!bytes.contains(&0), "cli: working directory contains NUL");
                Ok(std::ffi::OsString::from_vec(bytes.clone()).into())
            }
            #[cfg(windows)]
            Self::Windows(units) => {
                use std::os::windows::ffi::OsStringExt;
                ensure!(!units.contains(&0), "cli: working directory contains NUL");
                Ok(std::ffi::OsString::from_wide(units).into())
            }
            _ => bail!("cli: working directory belongs to a different platform"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionInfo {
    pub id: String,
    pub window_id: String,
    pub name: String,
    pub cwd: String,
    pub tab_count: usize,
    pub selected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Listed { sessions: Vec<SessionInfo> },
    Created { id: String },
    Applied {},
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    NotFound,
    ConfirmationRequired,
    OperationFailed,
}

pub fn read_frame<T: DeserializeOwned>(reader: &mut impl Read) -> anyhow::Result<T> {
    let mut length = [0; 4];
    reader
        .read_exact(&mut length)
        .context("cli: reading frame length")?;
    let length = u32::from_be_bytes(length) as usize;
    ensure!(
        length > 0 && length <= MAX_FRAME,
        "cli: invalid frame length {length}"
    );
    let mut bytes = vec![0; length];
    reader
        .read_exact(&mut bytes)
        .context("cli: reading frame body")?;
    serde_json::from_slice(&bytes).context("cli: invalid protocol JSON")
}

pub fn write_frame<T: Serialize>(writer: &mut impl Write, value: &T) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(value).context("cli: encoding protocol JSON")?;
    ensure!(
        bytes.len() <= MAX_FRAME,
        "cli: frame exceeds {MAX_FRAME} bytes"
    );
    writer
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .context("cli: writing frame length")?;
    writer
        .write_all(&bytes)
        .context("cli: writing frame body")?;
    writer.flush().context("cli: flushing frame")
}

#[cfg(test)]
mod tests;
