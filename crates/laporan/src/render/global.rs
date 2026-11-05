//! Rendering proxy to a global report [`Renderer`].

use std::{
    fmt::{self, Display},
    sync::OnceLock,
};

use crate::{
    RawReport,
    render::{Renderer, SimpleRenderer},
};

static RENDERER: OnceLock<Box<dyn DynRender + Send + Sync>> = OnceLock::new();

trait DynRender {
    fn render(&self, raw: &RawReport, fmt: &mut fmt::Formatter) -> fmt::Result;
}

impl<T: Renderer> DynRender for T {
    #[inline]
    fn render(&self, raw: &RawReport, f: &mut fmt::Formatter) -> fmt::Result {
        Renderer::render(self, raw).fmt(f)
    }
}

struct GlobalDisplayer<'a> {
    raw: &'a RawReport,
}

impl fmt::Display for GlobalDisplayer<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let renderer = match RENDERER.get() {
            Some(global) => global.as_ref(),
            None => &SimpleRenderer,
        };

        renderer.render(self.raw, f)
    }
}

/// Proxy renderer to the global [`Renderer`], set by [`GlobalRenderer::set`].
/// If there is no global renderer registered, the impl falls back to using a [`SimpleRenderer`].
#[derive(Debug, Copy, Clone, Default)]
pub struct GlobalRenderer;

impl GlobalRenderer {
    /// Sets the global [`Renderer`].
    /// The global renderer can only be set once and applies to the entire program,
    /// so libraries should avoid calling this function.
    ///
    /// Returns `None` if a renderer was already set.
    #[inline]
    pub fn set(renderer: impl Renderer + Send + Sync + 'static) -> Option<()> {
        RENDERER.set(Box::new(renderer)).ok()
    }
}

impl Renderer for GlobalRenderer {
    #[inline]
    fn render(&self, raw: &RawReport) -> impl fmt::Display {
        GlobalDisplayer { raw }
    }
}
