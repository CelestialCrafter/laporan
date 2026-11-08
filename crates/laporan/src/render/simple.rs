//! Default rendering for [`crate::Report`]s.

use core::fmt;

use crate::{RawReport, render::Renderer};

/// Default renderer for [`crate::Report`]s.
///
/// [`crate::Report`]s can be rendered via the [`Renderer`] trait.
#[derive(Debug, Copy, Clone, Default)]
pub struct SimpleRenderer;

impl SimpleRenderer {
    /// Constructs a new `SimpleRenderer`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Renderer for SimpleRenderer {
    #[inline]
    fn render(&self, report: &RawReport) -> impl fmt::Display {
        &report.message
    }
}
