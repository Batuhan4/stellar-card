use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    General,
    Usage,
    AuthenticationFailure,
    InsufficientBalance,
    ResourceNotFound,
    RateLimited,
    ExternalService,
    Network,
    DepositPending,
}

impl ErrorCode {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::General => 1,
            Self::Usage => 2,
            Self::AuthenticationFailure => 3,
            Self::InsufficientBalance => 4,
            Self::ResourceNotFound => 5,
            Self::RateLimited => 6,
            Self::ExternalService => 7,
            Self::Network => 8,
            Self::DepositPending => 10,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "GENERAL_ERROR",
            Self::Usage => "INVALID_ARGUMENTS",
            Self::AuthenticationFailure => "AUTHENTICATION_FAILURE",
            Self::InsufficientBalance => "INSUFFICIENT_BALANCE",
            Self::ResourceNotFound => "RESOURCE_NOT_FOUND",
            Self::RateLimited => "RATE_LIMITED",
            Self::ExternalService => "EXTERNAL_SERVICE_ERROR",
            Self::Network => "NETWORK_ERROR",
            Self::DepositPending => "DEPOSIT_PENDING",
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Value,
    pub suggestion: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: json!({}),
            suggestion: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::new(ErrorCode::General, value.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(value: toml::de::Error) -> Self {
        Self::new(ErrorCode::General, value.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(value: toml::ser::Error) -> Self {
        Self::new(ErrorCode::General, value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(ErrorCode::General, value.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        let code = if value.is_connect() || value.is_timeout() {
            ErrorCode::Network
        } else {
            ErrorCode::ExternalService
        };
        Self::new(code, value.to_string())
    }
}
