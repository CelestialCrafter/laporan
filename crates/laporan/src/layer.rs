//! Utilities for interoperating with the [`tracing`] crate.

use std::{
    any::{Any, type_name},
    error::Error,
    fmt,
    io::Write,
};

use tracing::{Event, Span, Subscriber, field, span};
use tracing_subscriber::{
    Layer, fmt::MakeWriter, layer::Context as LayerContext, registry::LookupSpan,
};

use crate::{
    Level, Location, Metadata, Report, ReportError, ReportPayload,
    frame::{Attachment, Context, DynFrame, Suggestion},
    render::{GlobalRenderer, Renderer},
};

#[derive(Default)]
struct RecordVisitor {
    message: Option<String>,
    frames: Vec<Box<dyn DynFrame>>,
    children: Vec<ReportPayload>,
}

impl RecordVisitor {
    fn record(&mut self, field: &field::Field, value: impl fmt::Display) {
        if field.name().starts_with("log.") {
            return;
        }

        match field.name() {
            "message" => self.message = Some(value.to_string()),
            "suggestion" => self.frames.push(Box::new(Suggestion::new(value))),
            "attachment" => self.frames.push(Box::new(Attachment::new(value))),
            key => self.frames.push(Box::new(Context::new(key, value))),
        }
    }
}

impl field::Visit for RecordVisitor {
    fn record_str(&mut self, field: &field::Field, value: &str) {
        self.record(field, value)
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        self.record(field, format_args!("{value:?}"))
    }

    fn record_error(&mut self, _: &field::Field, value: &(dyn Error + 'static)) {
        self.children
            .push(match value.downcast_ref::<ReportError>() {
                Some(ReportError(payload)) => payload.clone(),
                None => Report::from_error(value).into_payload(),
            });
    }
}

fn metadata_to_payload(metadata: &tracing::Metadata, visitor: RecordVisitor) -> ReportPayload {
    let type_name = if metadata.is_span() {
        type_name::<Span>()
    } else {
        type_name::<Event>()
    };

    let message = if metadata.is_span() {
        metadata.name().to_string()
    } else {
        visitor
            .message
            .unwrap_or_else(|| "no message specified".to_string())
    };

    let level = match *metadata.level() {
        tracing::Level::TRACE => Level::Trace,
        tracing::Level::DEBUG => Level::Debug,
        tracing::Level::INFO => Level::Info,
        tracing::Level::WARN => Level::Warn,
        tracing::Level::ERROR => Level::Error,
    };

    let location = match metadata.file() {
        Some(name) => Location::File {
            name: name.to_string(),
            line: metadata.line(),
            column: None,
        },
        None => Location::Module(metadata.target().to_string()),
    };

    ReportPayload {
        frames: visitor.frames,
        children: visitor.children,
        message,
        metadata: Metadata {
            location,
            level,
            type_name,
        },
    }
}

/// A [`Layer`] that prints [`Event`]s to a report [`Renderer`].
pub struct ReportLayer<R, W> {
    renderer: R,
    writer: W,
}

impl ReportLayer<GlobalRenderer, fn() -> std::io::Stderr> {
    /// Constructs a new `ReportLayer`.
    /// Uses the `GlobalRenderer` and writes to `std::io::stderr` by default.
    #[inline]
    #[must_use]
    #[expect(
        clippy::new_without_default,
        reason = "there is no meaningful \"zeroed\" state for a `ReportLayer`"
    )]
    pub fn new() -> Self {
        Self {
            renderer: GlobalRenderer,
            writer: std::io::stderr,
        }
    }
}

impl<R, W> ReportLayer<R, W> {
    /// Sets the [`Renderer`] this layer uses to display reports.
    #[inline]
    pub fn with_renderer<T>(self, renderer: T) -> ReportLayer<T, W> {
        ReportLayer {
            renderer,
            writer: self.writer,
        }
    }

    /// Sets the [`MakeWriter`] this layer uses to write events.
    #[inline]
    pub fn with_writer<X>(self, writer: X) -> ReportLayer<R, X> {
        ReportLayer {
            renderer: self.renderer,
            writer,
        }
    }
}

impl<S, R, W> Layer<S> for ReportLayer<R, W>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    R: Renderer + 'static,
    W: for<'a> MakeWriter<'a> + 'static,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: LayerContext<'_, S>) {
        let Some(span) = ctx.span(id) else {
            return;
        };

        let mut visitor = RecordVisitor::default();
        attrs.record(&mut visitor);

        let payload = metadata_to_payload(span.metadata(), visitor);
        // NOTE: `insert` panics when a payload is already present, breaking the use of multiple `ReportLayer`s
        let _ = span.extensions_mut().replace(payload);
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: LayerContext<'_, S>) {
        let Some(span) = ctx.span(span) else {
            return;
        };

        let mut extensions = span.extensions_mut();
        let Some(payload) = extensions.get_mut::<ReportPayload>() else {
            return;
        };

        let mut visitor = RecordVisitor::default();
        values.record(&mut visitor);

        for new in visitor.frames {
            // preparation for upserting context frames with new values
            payload.frames.retain(|old| {
                let new = (new.as_ref() as &dyn Any).downcast_ref::<Context>();
                let old = (old.as_ref() as &dyn Any).downcast_ref::<Context>();
                new.zip(old).is_none_or(|(new, old)| new.key != old.key)
            });

            payload.frames.push(new);
        }

        payload.children.extend(visitor.children);
        if let Some(message) = visitor.message {
            payload.message = message;
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: LayerContext<'_, S>) {
        let metadata = event.metadata();

        let mut visitor = RecordVisitor::default();
        event.record(&mut visitor);

        let payload = metadata_to_payload(metadata, visitor);
        let payload = ctx
            .event_scope(event)
            .and_then(|scope| {
                let mut spans =
                    scope.filter_map(|span| span.extensions().get::<ReportPayload>().cloned());

                let mut root = spans.next()?;
                root.children.push(payload.clone());

                for mut parent in spans {
                    parent.children.push(root);
                    root = parent;
                }

                Some(root)
            })
            .unwrap_or(payload);

        let display = self.renderer.render(&payload);
        let mut writer = self.writer.make_writer_for(metadata);
        // NOTE: we can't actually return errors, so this is just best-effort behavior
        let _ = writeln!(writer, "{display}");
    }
}
