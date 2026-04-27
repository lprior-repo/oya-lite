use serde::{Deserialize, Serialize};

macro_rules! define_string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            #[allow(dead_code)]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

define_string_newtype!(WorkspaceName);
define_string_newtype!(WorkspacePath);
define_string_newtype!(BookmarkName);
define_string_newtype!(StepName);
define_string_newtype!(OpencodeUrl);
define_string_newtype!(PromptString);
define_string_newtype!(ModelId);
define_string_newtype!(RepoUrl);
define_string_newtype!(Username);
define_string_newtype!(ErrorMessage);
define_string_newtype!(Timestamp);
define_string_newtype!(MoonTaskName);
define_string_newtype!(DataDirPath);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjArgs(pub Vec<String>);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveString(pub String);

impl std::fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SensitiveString(***)")
    }
}

impl SensitiveString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepResult {
    Success,
    Failure,
}

impl StepResult {
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}
