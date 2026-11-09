//! Rendering proxy to a global report [`Renderer`].

use alloc::boxed::Box;
use core::{
    fmt::{self, Display},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{
    RawReport,
    render::{Renderer, SimpleRenderer},
};

static RENDERER: AtomicPtr<&'static dyn DynRender> = AtomicPtr::new(ptr::null_mut());

trait DynRender: Send + Sync {
    fn render(&self, raw: &RawReport, fmt: &mut fmt::Formatter) -> fmt::Result;
}

impl<T: Renderer + Send + Sync> DynRender for T {
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let renderer = NonNull::new(RENDERER.load(Ordering::Acquire)).map(|ptr|
                // SAFETY: All pointers stored into `RENDERER` must originate from valid references that live for 'static.
                unsafe { ptr.as_ref() });

        match renderer {
            Some(renderer) => renderer.render(self.raw, formatter),
            None => Renderer::render(&SimpleRenderer, self.raw).fmt(formatter),
        }
    }
}

/// Proxy renderer to the global [`Renderer`], set by [`GlobalRenderer::set`].
/// If there is no global renderer registered, the impl falls back to using a [`SimpleRenderer`].
#[derive(Debug, Copy, Clone, Default)]
pub struct GlobalRenderer;

impl GlobalRenderer {
    /// Sets the global [`Renderer`].
    ///
    /// The global renderer applies to the entire program, so libraries should avoid calling this function.
    /// Renderers used must be `'static` references to a renderer, meaning that you probably should use [`Box::leak`] or something similar.
    #[inline]
    pub fn set<R: Renderer + Send + Sync>(renderer: &'static R) {
        let ptr = Box::into_raw(Box::new(renderer as _));
        RENDERER.swap(ptr, Ordering::AcqRel);
    }
}

impl Renderer for GlobalRenderer {
    #[inline]
    fn render(&self, raw: &RawReport) -> impl fmt::Display {
        GlobalDisplayer { raw }
    }
}
