use std::fmt;

use tracing::error;

/// # battery module Error Codes
///
/// Implements standard errors for the battery module
#[derive(Debug, Default, Clone, Copy)]
pub enum DeviceInfoServiceErrorCodes {
    #[default]
    UnknownError,
    GetDeviceInfoError,
}

impl fmt::Display for DeviceInfoServiceErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceInfoServiceErrorCodes::UnknownError => write!(f, "UnknownError"),
            DeviceInfoServiceErrorCodes::GetDeviceInfoError => write!(f, "GetDeviceInfoError"),
        }
    }
}

/// # DeviceInfoServiceError
///
/// Implements a standard error type for all status bar related errors
/// includes the error code (`DeviceInfoServiceErrorCodes`) and a message
#[derive(Debug, Default)]
pub struct DeviceInfoServiceError {
    pub code: DeviceInfoServiceErrorCodes,
    pub message: String,
}

impl DeviceInfoServiceError {
    pub fn new(code: DeviceInfoServiceErrorCodes, message: String, _capture_error: bool) -> Self {
        error!("Error: (code: {:?}, message: {})", code, message);
        Self { code, message }
    }
}

impl std::fmt::Display for DeviceInfoServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}
