//! Error handling crate, split off from Xuehua

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

extern crate alloc;

pub mod error;
pub mod ext;
pub mod frame;
#[cfg(feature = "tracing")]
#[cfg_attr(docsrs, doc(cfg(feature = "tracing")))]
pub mod layer;
pub mod prelude;
pub mod render;
pub mod util;

#[doc(hidden)]
pub mod __private {
    pub extern crate alloc;
}

use core::{
    any::type_name,
    error::Error,
    fmt::{self, Display},
    marker::PhantomData,
    panic::Location as StdLocation,
    result::Result as StdResult,
};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use educe::Educe;

#[cfg(feature = "std")]
use crate::render::GlobalRenderer;
use crate::{
    frame::{DynFrame, Frame},
    render::{Renderer, SimpleRenderer},
};

pub use ext::{ReportExt, ResultReportExt};
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use laporan_derive::IntoReport;

/// Helper alias for [`Error`]
pub type BoxDynError = Box<dyn Error + Send + Sync + 'static>;

/// Helper alias for [`core::result::Result`]
pub type Result<T, E> = StdResult<T, Report<E>>;
/// A [`Report`] with its error type erased.
pub type ErasedReport = Report<()>;
/// A [`Result`] with its error type erased.
pub type ErasedResult<T> = Result<T, ()>;

/// Location data associated with a [`Report`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Location {
    /// A location pointing to a file
    File {
        /// The name of the file
        name: String,
        /// Optionally, the line within the file
        line: Option<u32>,
        /// Optionally, the column within the line
        column: Option<u32>,
    },
    /// A location pointing to a module
    Module(String),
    /// An unknown locatiion
    #[default]
    Unknown,
}

/// Severity level associated with a [`Report`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Level {
    /// Very low priority and verbose information.
    Trace,
    /// Lower priority debug information.
    Debug,
    /// Useful information.
    Info,
    /// Potentially dangerous situations.
    Warn,
    /// Very serious errors.
    #[default]
    Error,
}

/// Metadata associated with a [`Report`].
#[derive(Clone, Debug, Educe)]
#[educe(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Metadata {
    /// The location in which the [`Report`] originated.
    pub location: Location,
    /// The importance level of the [`Report`]
    pub level: Level,
    /// The name of the type associated with the [`Report`]
    ///
    /// Note: This field has the same semantics as [`type_name`]
    #[educe(Default(expr = core::any::type_name::<()>()))]
    pub type_name: &'static str,
}

impl Metadata {
    #[track_caller]
    fn new() -> Self {
        let location = StdLocation::caller();

        Self {
            location: Location::File {
                name: location.file().to_string(),
                line: Some(location.line()),
                column: Some(location.column()),
            },
            type_name: type_name::<()>(),
            level: Level::Error,
        }
    }
}

/// Inner raw of a [`Report`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RawReport {
    frames: Vec<Box<dyn DynFrame>>,
    children: Vec<Self>,
    message: String,
    metadata: Metadata,
}

impl RawReport {
    /// Construct a new `RawReport`.
    #[track_caller]
    #[inline]
    #[must_use]
    pub fn new(message: impl fmt::Display) -> Self {
        Self {
            message: message.to_string(),
            metadata: Metadata::new(),
            children: Vec::default(),
            frames: Vec::default(),
        }
    }

    /// Returns the associated child reports.
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    /// Returns the associated report message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the associated report metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns an iterator over frames of type `T`.
    pub fn frames_of<T: 'static>(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.frames
            .iter()
            .filter_map(|frame| (frame.as_ref() as &dyn core::any::Any).downcast_ref())
    }
}

/// The core error type, representing a tree of errors.
///
/// Each report contains [`Frame`]s, child [`RawReport`]s,
/// and additional information about the error.
#[derive(Educe)]
#[educe(Deref, Clone(bound()))]
pub struct Report<E> {
    #[educe(Deref)]
    inner: Box<RawReport>,
    _marker: PhantomData<fn() -> E>,
}

impl ErasedReport {
    /// Constructs a new [`ErasedReport`] from a message.
    #[track_caller]
    #[inline]
    pub fn new(message: impl fmt::Display) -> Self {
        Self::from_raw(RawReport::new(message.to_string()))
    }
}

impl<E: Error> Report<E> {
    /// Constructs a new `Report` from an error.
    ///
    /// Note that this walks [`Error::source`] to build a tree of children.
    /// To avoid this behavior, consider using [`ErasedReport::new`].
    #[track_caller]
    #[inline]
    pub fn from_error(error: E) -> Self {
        fn walk(error: &dyn Error) -> ErasedReport {
            let mut report = Report::new(error.to_string());
            report.inner.metadata.location = Location::Unknown;

            match error.source() {
                Some(source) => report.with_child(walk(source)),
                None => report,
            }
        }

        let mut report = Report::new(error.to_string()).cast();
        if let Some(source) = error.source() {
            report = report.with_child(walk(source));
        }

        report
    }
}

impl<E> Report<E> {
    /// Converts the underlying [`RawReport`] back into a `Report`.
    #[inline]
    #[must_use]
    pub fn from_raw(raw: RawReport) -> Self {
        Self {
            inner: raw.into(),
            _marker: PhantomData,
        }
    }

    /// Converts the `Report` into the underlying [`RawReport`].
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> RawReport {
        *self.inner
    }

    /// Converts this `Report` into an [`ErasedReport`].
    ///
    /// Note: This does not change the stored [`Metadata::type_name`], unlike [`Report::cast`].
    #[inline]
    #[must_use]
    pub fn erased(self) -> ErasedReport {
        Report {
            inner: self.inner,
            _marker: PhantomData,
        }
    }

    /// Replaces the message associated with this `Report`.
    #[inline]
    #[must_use]
    pub fn with_message(mut self, message: impl fmt::Display) -> Self {
        self.inner.message = message.to_string();
        self
    }

    /// Sets the level associated with this `Report`.
    #[inline]
    #[must_use]
    pub fn with_level(mut self, level: Level) -> Self {
        self.inner.metadata.level = level;
        self
    }

    /// Sets the location associated with this `Report`.
    #[inline]
    #[must_use]
    pub fn with_location(mut self, location: Location) -> Self {
        self.inner.metadata.location = location;
        self
    }

    /// Appends a [`Frame`] to this `Report`.
    #[inline]
    #[must_use]
    pub fn with_frame(mut self, frame: impl Frame) -> Self {
        self.inner.frames.push(Box::new(frame));
        self
    }

    /// Appends an iterator of [`Frame`]s to this `Report`.
    #[must_use]
    pub fn with_frames(mut self, frames: impl IntoIterator<Item = impl Frame>) -> Self {
        let iter = frames.into_iter().map(|frame| Box::new(frame) as Box<_>);
        self.inner.frames.extend(iter);
        self
    }

    /// Appends a `Report` as a child to this `Report`.
    #[inline]
    #[must_use]
    pub fn with_child<F>(mut self, child: Report<F>) -> Self {
        self.inner.children.push(*child.inner);
        self
    }

    /// Appends an iterator of `Report`s as children of this `Report`.
    #[inline]
    #[must_use]
    pub fn with_children<F>(mut self, children: impl IntoIterator<Item = Report<F>>) -> Self {
        let iter = children.into_iter().map(|child| *child.inner);
        self.inner.children.extend(iter);
        self
    }

    /// "Casts" the reports generic parameter to `F`.
    ///
    /// It's usually not recommended to use this function, and instead use [`Report::erased`].
    /// This is because `cast` changes the underlying [`Metadata::type_name`] associated with the `Report`.
    #[inline]
    #[must_use]
    pub fn cast<F>(mut self) -> Report<F> {
        self.inner.metadata.type_name = type_name::<F>();

        Report {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<E> AsRef<RawReport> for Report<E> {
    #[inline]
    fn as_ref(&self) -> &RawReport {
        self.inner.as_ref()
    }
}

impl<E> From<RawReport> for Report<E> {
    fn from(value: RawReport) -> Self {
        Self::from_raw(value)
    }
}

impl<E> From<Report<E>> for RawReport {
    fn from(value: Report<E>) -> Self {
        value.into_raw()
    }
}

#[cfg_attr(
    feature = "std",
    doc = "If the `std` feature is *enabled*, [`GlobalRenderer`] is used for the [`fmt::Debug`] Implementation."
)]
#[cfg_attr(
    not(feature = "std"),
    doc = "If the `std` feature is *enabled*, the global renderer is used for the [`fmt::Debug`] Implementation."
)]
/// Otherwise, a [`SimpleRenderer`] is used.
/// Additionally, the `{:#?}` directive can show the `Report` in a typical [`fmt::Debug`] fashion.
impl<E> fmt::Debug for Report<E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.inner.fmt(f)
        } else {
            #[cfg(feature = "std")]
            {
                GlobalRenderer.render(self).fmt(f)
            }
            #[cfg(not(feature = "std"))]
            {
                SimpleRenderer.render(self).fmt(f)
            }
        }
    }
}

impl<E> fmt::Display for Report<E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SimpleRenderer.render(self).fmt(f)
    }
}

/// Trait for converting [`Error`]s into enriched [`Report`]s
///
/// This can be implemented via `#[derive(IntoReport)]`.
#[cfg_attr(
    feature = "derive",
    doc = "See [`laporan_derive`] for more information."
)]
#[cfg_attr(
    not(feature = "derive"),
    doc = "See the `laporan_derive` crate for more information."
)]
pub trait IntoReport: Sized + Send + Sync + 'static {
    /// Converts this Error into a [`Report`].
    ///
    /// Consumers should prefer `into_report` over [`Report::new`]
    /// to generate pre-enriched reports.
    ///
    /// Implementations should modify the [`Report`]
    /// (by adding frames, children, etc) as they see fit.
    #[track_caller]
    fn into_report(self) -> Report<Self>;
}

impl<T: IntoReport> From<T> for Report<T> {
    #[track_caller]
    #[inline]
    fn from(value: T) -> Self {
        value.into_report()
    }
}
