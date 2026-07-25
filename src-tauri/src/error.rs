use serde::Serialize;

/// Every fallible path in the app funnels through this type so the frontend
/// always receives a readable message instead of a debug-formatted blob.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Invalid(String),

    #[error("{0} was not found")]
    NotFound(String),

    #[error("{0}")]
    Runtime(String),
}

impl AppError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound(what.into())
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }

    /// Short machine-readable kind, handy for the frontend when it wants to
    /// react differently to a missing record than to a validation failure.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Invalid(_) => "invalid",
            Self::NotFound(_) => "not_found",
            Self::Runtime(_) => "runtime",
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        Self::Runtime(value.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct SerializedError {
    kind: &'static str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializedError {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
