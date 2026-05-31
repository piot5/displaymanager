use snafu::prelude::*;

/// Global error type for DDC/CI operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DdcError {
    #[snafu(display("DDC/CI backend not available: {details}"))]
    BackendNotAvailable { details: String },

    #[snafu(display("Hardware communication failed: {reason}"))]
    CommunicationFailed { reason: String },

    #[snafu(display("Access denied: Check I2C group permissions or Admin rights."))]
    AccessDenied,

    #[snafu(display("The requested VCP feature is not supported by this monitor."))]
    UnsupportedFeature,

    #[snafu(display("Invalid DDC/CI device at {path}"))]
    InvalidDevice { path: String },
}