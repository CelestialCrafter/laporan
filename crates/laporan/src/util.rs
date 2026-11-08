//! Utility functions and macros.

use core::{iter::once, result::Result as StdResult};

/// Defines a Compat trait for populating common error types.
#[macro_export]
macro_rules! impl_compat {
    ($name:ident: [$(|$argument:ident: $error:path| $block:expr),*]) => {
        $crate::impl_compat!(@define $name);

        $(impl<T> $name<T, $error> for ::core::result::Result<T, $error> {
            #[track_caller]
            #[inline]
            fn compat(self) -> ::core::result::Result<T, $crate::Report<$error>> {
                #[allow(unused_variables)]
                #[track_caller]
                fn convert($argument: $error) -> $crate::Report<$error> {
                    $block
                }

                // we can't use [`Result::map_err`] since `#[track_caller]` doesn't work with it
                match self {
                    Ok(ok) => Ok(ok),
                    Err(error) => Err(convert(error))
                }
            }
        })*
    };
    ($name:ident: [$($error:path),*]) => {
        $crate::impl_compat!(@define $name);

        $(impl<T> $name<T, $error> for ::core::result::Result<T, $error> {
            #[track_caller]
            #[inline]
            fn compat(self) -> ::core::result::Result<T, $crate::Report<$error>> {
                match self {
                    Ok(ok) => Ok(ok),
                    Err(error) => Err($crate::Report::from_error(error))
                }
            }
        })*
    };
    (@define $name:ident) => {
        /// Helper trait for populating common error types.
        pub trait $name<T, E>: Sized {
            /// Converts this error into a pre-populated [`crate::Report`].
            #[must_use]
            fn compat(self) -> ::core::result::Result<T, $crate::Report<E>>;
        }
    };
}

/// Helper function to partition an [`Iterator`] based on its [`Result`].
///
/// If value is of the [`Result::Err`], all errors are returned and the collected [`Result::Ok`] values are discarded.
pub fn partition_results<T, U, E, F>(
    iterator: impl Iterator<Item = StdResult<T, E>>,
) -> StdResult<U, F>
where
    U: Extend<T> + Default,
    F: Extend<E> + Default,
{
    let mut ok = U::default();
    let mut error = F::default();

    let has_error = iterator.fold(false, |has_error, result| match result {
        Ok(value) => {
            ok.extend(once(value));
            has_error
        }
        Err(value) => {
            error.extend(once(value));
            true
        }
    });

    if has_error { Err(error) } else { Ok(ok) }
}
