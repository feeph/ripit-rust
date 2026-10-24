/*
    provide domain-specific error values

    Some MakeMkv errors are irrelevant, e.g. it's not possible to return a
    content-related error during backup since `makemkvcon backup` does not
    scan the content.
*/

// standard library imports
// <none>

// third-party imports
// <none>

// crate-provided imports
use crate::runner::{MakeMkvErrorContext, MakeMkvErrorKind};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub enum BackupError {
    InitError(InitErrorKind),
    DataError(String),
    UnknownError,
}

impl From<MakeMkvError> for BackupError {
    fn from(err: MakeMkvError) -> Self {
        match err {
            MakeMkvError::InitError(kind) => BackupError::InitError(kind),
            MakeMkvError::DataError(x) => BackupError::DataError(x),
            MakeMkvError::UnknownError => BackupError::UnknownError,
        }
    }
}

pub enum InfoError {
    InitError(InitErrorKind),
    DataError(String),
    UnknownError,
}

impl From<MakeMkvError> for InfoError {
    fn from(err: MakeMkvError) -> Self {
        match err {
            MakeMkvError::InitError(kind) => InfoError::InitError(kind),
            MakeMkvError::DataError(x) => InfoError::DataError(x),
            MakeMkvError::UnknownError => InfoError::UnknownError,
        }
    }
}

pub enum MkvError {
    InitError(InitErrorKind),
    DataError(String),
    UnknownError,
}

impl From<MakeMkvError> for MkvError {
    fn from(err: MakeMkvError) -> Self {
        match err {
            MakeMkvError::InitError(kind) => MkvError::InitError(kind),
            MakeMkvError::DataError(x) => MkvError::DataError(x),
            MakeMkvError::UnknownError => MkvError::UnknownError,
        }
    }
}

// TODO review enum FirmwareError; 'DataError' doesn't make sense in this context
pub enum FirmwareError {
    InitError(InitErrorKind),
    DataError(String),
    UnknownError,
}

impl From<MakeMkvError> for FirmwareError {
    fn from(err: MakeMkvError) -> Self {
        match err {
            MakeMkvError::InitError(kind) => FirmwareError::InitError(kind),
            MakeMkvError::DataError(x) => FirmwareError::DataError(x),
            MakeMkvError::UnknownError => FirmwareError::UnknownError,
        }
    }
}

// TODO review enum LicenseError; 'DataError' doesn't make sense in this context
pub enum LicenseError {
    InitError(InitErrorKind),
    DataError(String),
    UnknownError,
}

impl From<MakeMkvError> for LicenseError {
    fn from(err: MakeMkvError) -> Self {
        match err {
            MakeMkvError::InitError(kind) => LicenseError::InitError(kind),
            MakeMkvError::DataError(x) => LicenseError::DataError(x),
            MakeMkvError::UnknownError => LicenseError::UnknownError,
        }
    }
}
