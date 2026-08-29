//! Tipo de erro usado pela lógica de servidor.
//!
//! Antes da migração para Axum nativo, a lógica usava `dioxus::prelude::ServerFnError`.
//! Mantemos a mesma forma (`ServerError { message }`) para preservar a lógica de negócio
//! intacta; a camada HTTP ([crate::api]) traduz isso em status + JSON.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerFnError {
    ServerError { message: String },
}

impl ServerFnError {
    pub fn new(message: impl Into<String>) -> Self {
        ServerFnError::ServerError {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ServerFnError::ServerError { message } => message,
        }
    }
}

impl std::fmt::Display for ServerFnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ServerFnError {}
