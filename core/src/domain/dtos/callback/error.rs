use std::fmt::Display;

#[derive(Debug)]
pub enum CallbackError {
    HttpError(String),
    ScriptError(String),
    ConfigError(String),
    IoError(std::io::Error),
}

impl Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(e) => write!(f, "HTTP callback failed: {}", e),
            Self::ScriptError(e) => write!(f, "Script execution failed: {}", e),
            Self::ConfigError(e) => write!(f, "Configuration error: {}", e),
            Self::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl CallbackError {
    pub fn http<S: Into<String>>(msg: S) -> Self {
        CallbackError::HttpError(msg.into())
    }

    pub fn script<S: Into<String>>(msg: S) -> Self {
        CallbackError::ScriptError(msg.into())
    }

    pub fn config<S: Into<String>>(msg: S) -> Self {
        CallbackError::ConfigError(msg.into())
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CallbackError::HttpError(_) | CallbackError::IoError(_)
        )
    }
}
