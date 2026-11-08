use core::{error::Error, fmt};

use crate::{
    RawReport, Report,
    render::{Renderer, SimpleRenderer},
};

/// Type representing a [`Report`], but implementing [`Error`].
///
/// This error type does not carry over all the frills of a [`Report`] (such as error trees).
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ReportError(RawReport);

impl ReportError {
    /// Constructs a new `ReportError` from a [`Report`].
    #[inline]
    #[track_caller]
    pub fn new<E>(report: impl Into<Report<E>>) -> Self {
        Self(*report.into().inner)
    }

    /// Converts this error back into a [`Report`].
    #[inline]
    #[must_use]
    pub fn into_inner<E>(self) -> Report<E> {
        Report::from_raw(self.0)
    }
}

impl AsRef<RawReport> for ReportError {
    fn as_ref(&self) -> &RawReport {
        &self.0
    }
}

impl Error for ReportError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // NOTE: we want to provide a source if there is *exactly* one child.
        //       in the case where there are multiple children, providing no information is better than providing misleading information.
        if self.0.children.len() != 1 {
            return None;
        }

        let raw = &self.0.children[0];
        // SAFETY: `Self` is repr(transparent) over `RawReport`.
        // SAFETY: The pointer we are dereferencing came from a valid reference.
        let error = unsafe {
            core::ptr::from_ref::<RawReport>(raw)
                .cast::<ReportError>()
                .as_ref_unchecked()
        };

        Some(error as &(dyn Error + 'static))
    }
}

impl fmt::Display for ReportError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SimpleRenderer.render(&self.0).fmt(f)
    }
}

impl<E> From<ReportError> for Report<E> {
    #[inline]
    fn from(value: ReportError) -> Self {
        value.into_inner()
    }
}

impl<E> From<Report<E>> for ReportError {
    #[inline]
    fn from(value: Report<E>) -> Self {
        Self::new(value)
    }
}
