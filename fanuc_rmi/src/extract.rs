//! Generic extraction trait for enum variants.
//!
//! This module provides the [`ExtractInner`] trait and supporting macros to enable
//! type-safe extraction of inner types from response enums.
//!
//! # Example
//!
//! ```
//! use fanuc_rmi::ExtractInner;
//! use fanuc_rmi::packets::CommandResponse;
//! use fanuc_rmi::commands::FrcReadJointAnglesResponse;
//!
//! # fn example(response: CommandResponse) {
//! // Extract by type - compiler infers T from context
//! let resp: Option<&FrcReadJointAnglesResponse> = response.as_inner();
//! if let Some(resp) = resp {
//!     // Use resp...
//!     println!("Error ID: {}", resp.error_id);
//! }
//! # }
//! ```

/// Trait for extracting inner types from enums.
///
/// This trait enables type-safe extraction of specific variant types from
/// response enums without manual pattern matching. It's implemented for
/// all response enum variants using the [`impl_extract_inner!`] macro.
///
/// # Type Parameters
///
/// * `T` - The inner type to extract from the enum
///
/// # Examples
///
/// ```
/// use fanuc_rmi::ExtractInner;
/// use fanuc_rmi::packets::CommandResponse;
/// use fanuc_rmi::commands::FrcReadJointAnglesResponse;
///
/// # fn example(response: CommandResponse) {
/// // Borrowing the inner value
/// let resp: Option<&FrcReadJointAnglesResponse> = response.as_inner();
/// if let Some(resp) = resp {
///     println!("Error ID: {}", resp.error_id);
/// }
///
/// // Taking ownership of the inner value
/// let owned: Option<FrcReadJointAnglesResponse> = response.into_inner();
/// if let Some(resp) = owned {
///     // resp is now owned
/// }
/// # }
/// ```
pub trait ExtractInner<T> {
    /// Extract a reference to the inner type, if present.
    ///
    /// Returns `Some(&T)` if the enum variant matches the requested type,
    /// otherwise returns `None`.
    fn as_inner(&self) -> Option<&T>;

    /// Extract the inner type by value, if present.
    ///
    /// Returns `Some(T)` if the enum variant matches the requested type,
    /// otherwise returns `None`. This consumes the enum.
    fn into_inner(self) -> Option<T>;

    /// Extract a reference to the inner type, panicking if not present.
    ///
    /// # Panics
    ///
    /// Panics with the provided message if the enum variant does not match
    /// the requested type.
    ///
    /// # Examples
    ///
    /// ```
    /// use fanuc_rmi::ExtractInner;
    /// use fanuc_rmi::packets::CommandResponse;
    /// use fanuc_rmi::commands::{FrcReadJointAnglesResponse, FrcAbortResponse};
    ///
    /// # fn example(response: CommandResponse) {
    /// // This will panic if response is not FrcReadJointAngles variant
    /// let resp: &FrcAbortResponse = response.expect_inner(
    ///     "Expected FrcAbort response"
    /// );
    /// # }
    /// ```
    #[inline]
    fn expect_inner(&self, msg: &str) -> &T {
        self.as_inner().expect(msg)
    }
}

/// Macro to implement [`ExtractInner`] for enum variants.
///
/// This macro generates the boilerplate implementation of the `ExtractInner`
/// trait for a specific enum variant and inner type combination.
///
/// # Arguments
///
/// * `$enum` - The enum type (e.g., `CommandResponse`)
/// * `$variant` - The variant name (e.g., `FrcReadJointAngles`)
/// * `$inner` - The inner type (e.g., `FrcReadJointAnglesResponse`)
///
/// # Examples
///
/// ```ignore
/// impl_extract_inner!(CommandResponse, FrcReadJointAngles, FrcReadJointAnglesResponse);
/// ```
#[macro_export]
macro_rules! impl_extract_inner {
    ($enum:ty, $variant:ident, $inner:ty) => {
        impl $crate::ExtractInner<$inner> for $enum {
            #[inline]
            fn as_inner(&self) -> Option<&$inner> {
                match self {
                    Self::$variant(inner) => Some(inner),
                    _ => None,
                }
            }

            #[inline]
            fn into_inner(self) -> Option<$inner> {
                match self {
                    Self::$variant(inner) => Some(inner),
                    _ => None,
                }
            }
        }
    };
}

