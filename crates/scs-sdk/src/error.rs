use core::fmt;

use crate::sys;

pub type SdkResult<T = ()> = Result<T, SdkError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdkError {
    Unsupported,
    InvalidParameter,
    AlreadyRegistered,
    NotFound,
    UnsupportedType,
    NotNow,
    Generic,
    Unknown(sys::ScsResult),
}

impl SdkError {
    /// Converts an SDK result code into a Rust result.
    ///
    /// # Errors
    ///
    /// Returns the matching error variant for every non-zero SDK result code.
    pub const fn from_code(code: sys::ScsResult) -> SdkResult {
        match code {
            sys::SCS_RESULT_OK => Ok(()),
            sys::SCS_RESULT_UNSUPPORTED => Err(Self::Unsupported),
            sys::SCS_RESULT_INVALID_PARAMETER => Err(Self::InvalidParameter),
            sys::SCS_RESULT_ALREADY_REGISTERED => Err(Self::AlreadyRegistered),
            sys::SCS_RESULT_NOT_FOUND => Err(Self::NotFound),
            sys::SCS_RESULT_UNSUPPORTED_TYPE => Err(Self::UnsupportedType),
            sys::SCS_RESULT_NOT_NOW => Err(Self::NotNow),
            sys::SCS_RESULT_GENERIC_ERROR => Err(Self::Generic),
            value => Err(Self::Unknown(value)),
        }
    }

    #[must_use]
    pub const fn code(self) -> sys::ScsResult {
        match self {
            Self::Unsupported => sys::SCS_RESULT_UNSUPPORTED,
            Self::InvalidParameter => sys::SCS_RESULT_INVALID_PARAMETER,
            Self::AlreadyRegistered => sys::SCS_RESULT_ALREADY_REGISTERED,
            Self::NotFound => sys::SCS_RESULT_NOT_FOUND,
            Self::UnsupportedType => sys::SCS_RESULT_UNSUPPORTED_TYPE,
            Self::NotNow => sys::SCS_RESULT_NOT_NOW,
            Self::Generic => sys::SCS_RESULT_GENERIC_ERROR,
            Self::Unknown(value) => value,
        }
    }
}

impl fmt::Display for SdkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => formatter.write_str("unsupported SDK operation or version"),
            Self::InvalidParameter => formatter.write_str("invalid SDK parameter"),
            Self::AlreadyRegistered => formatter.write_str("SDK callback is already registered"),
            Self::NotFound => formatter.write_str("SDK event or channel was not found"),
            Self::UnsupportedType => formatter.write_str("SDK value type is not supported"),
            Self::NotNow => {
                formatter.write_str("SDK operation is not allowed in the current state")
            }
            Self::Generic => formatter.write_str("generic SDK error"),
            Self::Unknown(value) => write!(formatter, "unknown SDK result code {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_all_sdk_result_codes() {
        let cases = [
            (sys::SCS_RESULT_UNSUPPORTED, SdkError::Unsupported),
            (
                sys::SCS_RESULT_INVALID_PARAMETER,
                SdkError::InvalidParameter,
            ),
            (
                sys::SCS_RESULT_ALREADY_REGISTERED,
                SdkError::AlreadyRegistered,
            ),
            (sys::SCS_RESULT_NOT_FOUND, SdkError::NotFound),
            (sys::SCS_RESULT_UNSUPPORTED_TYPE, SdkError::UnsupportedType),
            (sys::SCS_RESULT_NOT_NOW, SdkError::NotNow),
            (sys::SCS_RESULT_GENERIC_ERROR, SdkError::Generic),
        ];

        for (code, expected) in cases {
            assert_eq!(SdkError::from_code(code), Err(expected));
            assert_eq!(expected.code(), code);
        }
    }
}
