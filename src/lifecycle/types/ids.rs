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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_id_parse_empty_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = BeadId::parse("");
        assert!(result.is_err());
        match result {
            Err(BeadIdParseError::Empty) => Ok(()),
            _ => Err("expected Empty error".into()),
        }
    }

    #[test]
    fn test_bead_id_parse_too_long_error() -> Result<(), Box<dyn std::error::Error>> {
        let long_id = "a".repeat(65);
        let result = BeadId::parse(&long_id);
        assert!(result.is_err());
        match result {
            Err(BeadIdParseError::TooLong { len: 65, max: 64 }) => Ok(()),
            _ => Err("expected TooLong error".into()),
        }
    }

    #[test]
    fn test_bead_id_parse_invalid_chars_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = BeadId::parse("valid-id_UPPERCASE");
        assert!(result.is_err());
        match result {
            Err(BeadIdParseError::InvalidChars) => Ok(()),
            _ => Err("expected InvalidChars error".into()),
        }
    }

    #[test]
    fn test_bead_id_parse_invalid_chars_space() -> Result<(), Box<dyn std::error::Error>> {
        let result = BeadId::parse("invalid id");
        assert!(result.is_err());
        match result {
            Err(BeadIdParseError::InvalidChars) => Ok(()),
            _ => Err("expected InvalidChars error".into()),
        }
    }

    #[test]
    fn test_bead_id_parse_max_length_edge() -> Result<(), Box<dyn std::error::Error>> {
        let max_id = "a".repeat(64);
        let result = BeadId::parse(&max_id)?;
        assert_eq!(result.as_str().len(), 64);
        Ok(())
    }

    #[test]
    fn test_bead_id_parse_just_under_max() -> Result<(), Box<dyn std::error::Error>> {
        let id = "a".repeat(63);
        let result = BeadId::parse(&id)?;
        assert_eq!(result.as_str().len(), 63);
        Ok(())
    }

    #[test]
    fn test_bead_id_parse_whitespace_only_trims_to_empty() -> Result<(), Box<dyn std::error::Error>> {
        let result = BeadId::parse("   ");
        assert!(result.is_err());
        match result {
            Err(BeadIdParseError::Empty) => Ok(()),
            _ => Err("expected Empty error".into()),
        }
    }

    #[test]
    fn test_bead_id_error_display_empty() {
        let err = BeadIdParseError::Empty;
        let s = format!("{err}");
        assert!(s.contains("empty"));
    }

    #[test]
    fn test_bead_id_error_display_too_long() {
        let err = BeadIdParseError::TooLong { len: 100, max: 64 };
        let s = format!("{err}");
        assert!(s.contains("100"));
        assert!(s.contains("64"));
    }

    #[test]
    fn test_bead_id_error_display_invalid_chars() {
        let err = BeadIdParseError::InvalidChars;
        let s = format!("{err}");
        assert!(s.contains("invalid"));
    }

    #[test]
    fn test_bead_data_from_bead_id() -> Result<(), Box<dyn std::error::Error>> {
        let id = BeadId::parse("test-bead")?;
        let data = BeadData::from_bead_id(id.clone());
        assert_eq!(data.bead_id, id);
        assert!(data.workspace.as_str().contains("test-bead"));
        assert!(data.workspace_path.as_str().contains("test-bead"));
        assert!(data.bookmark.as_str().contains("test-bead"));
        Ok(())
    }
}