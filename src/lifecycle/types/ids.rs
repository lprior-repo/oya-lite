use serde::{Deserialize, Serialize};

use super::{BookmarkName, WorkspaceName, WorkspacePath};

const MAX_BEAD_ID_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(input: &str) -> Result<Self, BeadIdParseError> {
        let normalized = input.trim();
        if normalized.is_empty() {
            return Err(BeadIdParseError::Empty);
        }
        if normalized.len() > MAX_BEAD_ID_LEN {
            return Err(BeadIdParseError::TooLong {
                len: normalized.len(),
                max: MAX_BEAD_ID_LEN,
            });
        }
        if !normalized.chars().all(is_bead_char) {
            return Err(BeadIdParseError::InvalidChars);
        }
        Ok(Self(normalized.to_owned()))
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn is_bead_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadIdParseError {
    Empty,
    TooLong { len: usize, max: usize },
    InvalidChars,
}

impl std::fmt::Display for BeadIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "bead id must not be empty"),
            Self::TooLong { len, max } => {
                write!(f, "bead id exceeds max length: {len} > {max}")
            }
            Self::InvalidChars => write!(f, "bead id contains invalid chars"),
        }
    }
}

impl std::error::Error for BeadIdParseError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadData {
    pub bead_id: BeadId,
    pub workspace: WorkspaceName,
    pub workspace_path: WorkspacePath,
    pub bookmark: BookmarkName,
}

impl BeadData {
    #[must_use]
    pub fn from_bead_id(bead_id: BeadId) -> Self {
        let id_str = bead_id.as_str();
        Self {
            workspace: WorkspaceName(format!("workspace-{id_str}")),
            workspace_path: WorkspacePath(format!("../{id_str}")),
            bookmark: BookmarkName(format!("bead-{id_str}")),
            bead_id,
        }
    }
}
