//! Rendering for [`crate::Report`]s in various formats.

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod global;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
#[cfg(feature = "pretty")]
#[cfg_attr(docsrs, doc(cfg(feature = "pretty")))]
pub mod pretty;
pub mod simple;

use core::fmt;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use global::GlobalRenderer;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub use json::JsonRenderer;
#[cfg(feature = "pretty")]
#[cfg_attr(docsrs, doc(cfg(feature = "pretty")))]
pub use pretty::PrettyRenderer;
pub use simple::SimpleRenderer;

use crate::RawReport;

/// Trait for rendering [`crate::Report`]s.
pub trait Renderer {
    /// Renders a [`crate::Report`] into an impl [`fmt::Display`]
    fn render<'a>(&'a self, raw: &'a RawReport) -> impl fmt::Display + 'a;
}
