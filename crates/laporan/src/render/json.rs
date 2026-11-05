//! [JSON](https://json.org/) rendering for [`Report`]s.

use core::fmt;

use crate::{RawReport, render::Renderer};

#[derive(Debug, Clone)]
struct JsonDisplayer<'a> {
    inner: &'a JsonRenderer,
    raw: &'a RawReport,
}

impl fmt::Display for JsonDisplayer<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let serialize = if self.inner.pretty {
            serde_json::to_string_pretty
        } else {
            serde_json::to_string
        };

        let string = serialize(self.raw).map_err(|_| fmt::Error)?;
        f.write_str(&string)
    }
}

/// [JSON](https://json.org/) renderer for [`Report`]s.
///
/// [`Report`]s can be rendered via the [`Report`] trait.
#[derive(Default, Debug, Copy, Clone)]
pub struct JsonRenderer {
    /// Whether or not the output is pretty-printed.
    pub pretty: bool,
}

impl JsonRenderer {
    /// Constructs a new `JsonRenderer`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { pretty: false }
    }
}

impl Renderer for JsonRenderer {
    #[inline]
    fn render<'a>(&'a self, raw: &'a RawReport) -> impl fmt::Display + 'a {
        JsonDisplayer { inner: self, raw }
    }
}
