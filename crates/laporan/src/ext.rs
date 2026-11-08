use core::{error::Error, result::Result as StdResult};

use crate::{ErasedResult, IntoReport, Level, Location, Report, Result, error, frame::Frame};

/// Helper trait for [`Report<T>`].
pub trait ReportExt<T> {
    /// "Wraps" this `Report` with the parent's default `Report`.
    ///
    /// See [`Report::wrap`] for more information.
    #[track_caller]
    fn wrap<U: IntoReport + Default>(self) -> Report<U>;

    /// "Wraps" this `Report` with a parent `Report`.
    ///
    /// See [`Report::wrap_with`] for more information.
    #[track_caller]
    fn wrap_with<U>(self, parent: impl Into<Report<U>>) -> Report<U>;
}

impl<T, U: Into<Report<T>>> ReportExt<T> for U {
    #[inline]
    fn wrap<V: IntoReport + Default>(self) -> Report<V> {
        V::default().into_report().with_child(self.into())
    }

    #[inline]
    fn wrap_with<V>(self, parent: impl Into<Report<V>>) -> Report<V> {
        parent.into().with_child(self.into())
    }
}

/// Helper trait for [`Result<T, Report<E>>`].
pub trait ResultReportExt<T, E>: Sized {
    /// Converts this [`Report`] into a type implementing [`Error`].
    ///
    /// See [`Report::into_error`] for more information.
    #[track_caller]
    fn into_error(self) -> StdResult<T, impl Error + Send + Sync + 'static>;

    /// Converts this `Report` into an [`ErasedReport`].
    ///
    /// See [`Report::erased`] for more information.
    #[track_caller]
    fn erased(self) -> ErasedResult<T>;

    /// "Wraps" this [`Report`] with the parent's default [`Report`].
    ///
    /// See [`Report::wrap`] for more information.
    #[track_caller]
    fn wrap<F: IntoReport + Default>(self) -> Result<T, F> {
        self.wrap_with_fn(|| F::default())
    }

    /// Append a [`Report`] as a parent of this [`Report`].
    ///
    /// See [`Report::wrap_with`] for more information.
    #[track_caller]
    fn wrap_with<F, G: Into<Report<F>>>(self, parent: G) -> Result<T, F> {
        self.wrap_with_fn(|| parent)
    }

    /// Append a [`Report`] as a parent of this [`Report`].
    ///
    /// See [`Report::wrap_with`] for more information.
    #[track_caller]
    fn wrap_with_fn<F, G: Into<Report<F>>>(self, func: impl FnOnce() -> G) -> Result<T, F>;

    /// Appends a [`Frame`] to this [`Report`].
    ///
    /// See [`Report::with_frame`] for more information.
    #[track_caller]
    fn with_frame<F: Frame>(self, frame: impl FnOnce() -> F) -> Result<T, E>;

    /// Sets the level associated with this [`Report`].
    ///
    /// See [`Report::with_level`] for more information.
    #[track_caller]
    fn with_level(self, level: Level) -> Result<T, E>;

    /// Sets the location associated with this [`Report`].
    ///
    /// See [`Report::with_location`] for more information.
    #[track_caller]
    fn with_location(self, location: Location) -> Result<T, E>;

    /// Appends an iterator of [`Frame`]s to this [`Report`].
    ///
    /// See [`Report::with_frames`] for more information.
    #[track_caller]
    fn with_frames<F: Frame>(self, frames: impl IntoIterator<Item = F>) -> Result<T, E>;

    /// Appends a [`Report`] as a child of this [`Report`].
    ///
    /// See [`Report::with_child`] for more information.
    #[track_caller]
    fn with_child<F>(self, child: Report<F>) -> Result<T, E>;

    /// Appends an iterator of [`Report`]s as children of this [`Report`].
    ///
    /// See [`Report::with_children`] for more information.
    #[track_caller]
    fn with_children<F>(self, children: impl IntoIterator<Item = Report<F>>) -> Result<T, E>;
}

// NOTE: we can't use `Result::map_err` in any of these since `#[track_caller]` on closures is unstable
// NOTE: ensure you use `Into::into` on reports before passing the report to any other function without `#[track_caller]` to avoid losing information
impl<T, E, D: Into<Report<E>>> ResultReportExt<T, E> for StdResult<T, D> {
    #[inline]
    fn into_error(self) -> StdResult<T, impl Error + Send + Sync + 'static> {
        match self {
            Ok(t) => Ok(t),
            Err(report) => Err(error::ReportError::new(report.into())),
        }
    }

    #[inline]
    fn erased(self) -> ErasedResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().erased()),
        }
    }

    #[inline]
    fn wrap_with_fn<F, G: Into<Report<F>>>(self, parent: impl FnOnce() -> G) -> Result<T, F> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(parent().into().with_child(report.into())),
        }
    }

    #[inline]
    fn with_frame<F: Frame>(self, frame: impl FnOnce() -> F) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_frame(frame())),
        }
    }

    #[inline]
    fn with_level(self, level: Level) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_level(level)),
        }
    }

    #[inline]
    fn with_location(self, location: Location) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_location(location)),
        }
    }

    #[inline]
    fn with_frames<F: Frame>(self, frames: impl IntoIterator<Item = F>) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_frames(frames)),
        }
    }

    #[inline]
    fn with_child<F>(self, child: Report<F>) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_child(child)),
        }
    }

    #[inline]
    fn with_children<F>(self, children: impl IntoIterator<Item = Report<F>>) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(report) => Err(report.into().with_children(children)),
        }
    }
}
