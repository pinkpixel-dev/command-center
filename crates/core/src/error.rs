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

    #[error("{0}")]
    Credential(String),

    #[error("AI features are turned off. Enable them in Settings and save first.")]
    AiDisabled,

    #[error("No OpenAI API key is stored. Add one in Settings first.")]
    AiNotConfigured,

    #[error("{0}")]
    AiAuth(String),

    #[error("{0}")]
    AiModel(String),

    #[error("{0}")]
    AiRateLimit(String),

    #[error("{0}")]
    AiNetwork(String),

    #[error("{0}")]
    AiResponse(String),

    #[error("OpenAI declined the request.")]
    AiRefusal,

    #[error("{0}")]
    AiIncomplete(String),

    #[error("{0}")]
    AiMalformed(String),

    #[error("OpenAI returned more data than this task allows.")]
    AiResponseTooLarge,

    #[error("Request cancelled.")]
    AiCancelled,
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

    pub fn credential(message: impl Into<String>) -> Self {
        Self::Credential(message.into())
    }

    pub fn ai_auth(message: impl Into<String>) -> Self {
        Self::AiAuth(message.into())
    }

    pub fn ai_model(message: impl Into<String>) -> Self {
        Self::AiModel(message.into())
    }

    pub fn ai_rate_limit(message: impl Into<String>) -> Self {
        Self::AiRateLimit(message.into())
    }

    pub fn ai_network(message: impl Into<String>) -> Self {
        Self::AiNetwork(message.into())
    }

    pub fn ai_response(message: impl Into<String>) -> Self {
        Self::AiResponse(message.into())
    }

    /// Short machine-readable kind, handy for the frontend when it wants to
    /// react differently to a missing record than to a validation failure.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Invalid(_) => "invalid",
            Self::NotFound(_) => "not_found",
            Self::Runtime(_) => "runtime",
            Self::Credential(_) => "credential",
            Self::AiDisabled => "ai_disabled",
            Self::AiNotConfigured => "ai_not_configured",
            Self::AiAuth(_) => "ai_auth",
            Self::AiModel(_) => "ai_model",
            Self::AiRateLimit(_) => "ai_rate_limit",
            Self::AiNetwork(_) => "ai_network",
            Self::AiResponse(_) => "ai_response",
            Self::AiRefusal => "ai_refusal",
            Self::AiIncomplete(_) => "ai_incomplete",
            Self::AiMalformed(_) => "ai_malformed",
            Self::AiResponseTooLarge => "ai_response_too_large",
            Self::AiCancelled => "ai_cancelled",
        }
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
