#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Validation,
    Workspace,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Terminal,
    Transient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LifecycleError {
    Terminal {
        category: FailureCategory,
        message: String,
    },
    Transient {
        category: FailureCategory,
        message: String,
    },
}

impl LifecycleError {
    #[must_use]
    pub fn terminal(category: FailureCategory, message: impl Into<String>) -> Self {
        Self::Terminal {
            category,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn transient(category: FailureCategory, message: impl Into<String>) -> Self {
        Self::Transient {
            category,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn class(&self) -> FailureClass {
        match self {
            Self::Terminal { .. } => FailureClass::Terminal,
            Self::Transient { .. } => FailureClass::Transient,
        }
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn category(&self) -> FailureCategory {
        match self {
            Self::Terminal { category, .. } | Self::Transient { category, .. } => *category,
        }
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Terminal { message, .. } | Self::Transient { message, .. } => message,
        }
    }
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terminal { category, message } => {
                write!(f, "terminal {category:?}: {message}")
            }
            Self::Transient { category, message } => {
                write!(f, "transient {category:?}: {message}")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_construction() {
        let err = LifecycleError::terminal(FailureCategory::Command, "something broke");
        assert!(err.is_terminal());
        assert_eq!(err.class(), FailureClass::Terminal);
        assert_eq!(err.category(), FailureCategory::Command);
        assert_eq!(err.message(), "something broke");
    }

    #[test]
    fn transient_construction() {
        let err = LifecycleError::transient(FailureCategory::Workspace, "retry later");
        assert!(!err.is_terminal());
        assert_eq!(err.class(), FailureClass::Transient);
        assert_eq!(err.category(), FailureCategory::Workspace);
        assert_eq!(err.message(), "retry later");
    }

    #[test]
    fn display_format_terminal() {
        let err = LifecycleError::terminal(FailureCategory::Validation, "bad input");
        let s = format!("{err}");
        assert!(s.contains("terminal"));
        assert!(s.contains("bad input"));
    }

    #[test]
    fn display_format_transient() {
        let err = LifecycleError::transient(FailureCategory::Command, "timeout");
        let s = format!("{err}");
        assert!(s.contains("transient"));
        assert!(s.contains("timeout"));
    }

    #[test]
    fn equality_works() {
        let a = LifecycleError::terminal(FailureCategory::Command, "x");
        let b = LifecycleError::terminal(FailureCategory::Command, "x");
        let c = LifecycleError::terminal(FailureCategory::Command, "y");
        let d = LifecycleError::transient(FailureCategory::Command, "x");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let err = LifecycleError::terminal(FailureCategory::Workspace, "disk full");
        let json = serde_json::to_string(&err)?;
        let back: LifecycleError = serde_json::from_str(&json)?;
        assert_eq!(err, back);
        Ok(())
    }

    #[test]
    fn failure_category_serde() -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&FailureCategory::Command)?;
        assert!(json.contains("command"));
        let back: FailureCategory = serde_json::from_str(&json)?;
        assert_eq!(back, FailureCategory::Command);
        Ok(())
    }
}
