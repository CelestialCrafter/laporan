//! Pretty rendering for [`Report`]s.

use std::fmt;

use educe::Educe;
use owo_colors::{OwoColorize, Style};

use crate::{Frame, Level, Location, Metadata, ReportPayload, render::Renderer};

/// Styles for each log level.
#[derive(Educe, Debug, Copy, Clone)]
#[educe(Default(expression = Self::new()))]
#[expect(missing_docs)]
pub struct LogStyles {
    pub error: Style,
    pub warn: Style,
    pub info: Style,
    pub debug: Style,
    pub trace: Style,
}

impl LogStyles {
    /// Constructs a new `LogStyles`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            error: Style::new().red(),
            warn: Style::new().yellow(),
            info: Style::new().blue(),
            debug: Style::new().magenta(),
            trace: Style::new().white(),
        }
    }
}

/// Styles for each component.
#[derive(Educe, Debug, Copy, Clone)]
#[educe(Default(expression = Self::new()))]
#[expect(missing_docs)]
pub struct Styles {
    pub guides: Style,
    pub context: Style,
    pub suggestion: Style,
    pub attachment: Style,
    pub location: Style,
    pub distracting: Style,
    pub log: LogStyles,
}

impl Styles {
    /// Constructs a new `Styles`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            guides: Style::new(),
            suggestion: Style::new().green(),
            context: Style::new().cyan(),
            attachment: Style::new().yellow(),
            location: Style::new().purple(),
            distracting: Style::new(),
            log: LogStyles::new(),
        }
    }
}

/// Display characters for each guide.
#[derive(Educe, Debug, Copy, Clone)]
#[educe(Default(expression = Self::new()))]
#[expect(missing_docs)]
pub struct Guides {
    pub line: &'static str,
    pub empty: &'static str,
    pub branch: &'static str,
    pub last_branch: &'static str,
}

impl Guides {
    /// Constructs a new `Guides`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            line: "│ ",
            empty: "  ",
            branch: "├─",
            last_branch: "╰─",
        }
    }
}

/// Display characters for each log header.
#[derive(Educe, Debug, Copy, Clone)]
#[educe(Default(expression = Self::new()))]
#[expect(missing_docs)]
pub struct LogHeaders {
    pub error: &'static str,
    pub warn: &'static str,
    pub info: &'static str,
    pub debug: &'static str,
    pub trace: &'static str,
}

impl LogHeaders {
    /// Constructs a new `LogHeaders`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            error: "(error)",
            warn: "(warn)",
            info: "(info)",
            debug: "(debug)",
            trace: "(trace)",
        }
    }
}

/// Display characters for each header.
#[derive(Educe, Debug, Copy, Clone)]
#[educe(Default(expression = Self::new()))]
#[expect(missing_docs)]
pub struct Headers {
    pub context: &'static str,
    pub suggestion: &'static str,
    pub attachment: &'static str,
    pub location: &'static str,
    pub log: LogHeaders,
}

impl Headers {
    #[inline]
    const fn new() -> Self {
        Self {
            context: "(context)",
            suggestion: "(suggestion)",
            attachment: "(attachment)",
            location: "(location)",
            log: LogHeaders::new(),
        }
    }
}

/// Configuration for [`PrettyRenderer`].
#[derive(Default, Debug, Copy, Clone)]
#[expect(missing_docs)]
pub struct Config {
    pub guides: Guides,
    pub headers: Headers,
    pub styles: Styles,
}

impl Config {
    /// Constructs a new `Config`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            guides: Guides::new(),
            headers: Headers::new(),
            styles: Styles::new(),
        }
    }
}

/// Pretty renderer for [`Report`]s.
///
/// [`Report`]s can be rendered via the [`Report`] trait.
// TODO: add color enabled field
#[derive(Default, Debug, Copy, Clone)]
pub struct PrettyRenderer {
    /// Configuration for this renderer
    config: Config,
}

impl PrettyRenderer {
    /// Constructs a new `PrettyRenderer`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            config: Config::new(),
        }
    }

    /// Replaces the underlying render configuration for this renderer.
    #[inline]
    pub const fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }
}

impl Renderer for PrettyRenderer {
    #[inline]
    fn render<'a>(&'a self, payload: &'a ReportPayload) -> impl fmt::Display + 'a {
        PrettyDisplayer {
            inner: self,
            payload,
        }
    }
}

struct LinePrinter<'a, 'b> {
    fmt: &'a mut fmt::Formatter<'b>,
    is_first: bool,
}

impl<'a, 'b> LinePrinter<'a, 'b> {
    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        if !self.is_first {
            self.fmt.write_str("\n")?;
        }

        self.is_first = false;
        self.fmt.write_fmt(args)
    }
}

#[derive(Debug, Copy, Clone)]
struct PrettyDisplayer<'a> {
    inner: &'a PrettyRenderer,
    payload: &'a ReportPayload,
}

impl fmt::Display for PrettyDisplayer<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut printer = LinePrinter {
            fmt: f,
            is_first: true,
        };

        self.render_report(
            &mut printer,
            self.payload,
            format_args!(""),
            format_args!(""),
        )
    }
}

impl PrettyDisplayer<'_> {
    fn render_report(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        payload: &ReportPayload,
        prefix: fmt::Arguments<'_>,
        next_prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let headers = &self.inner.config.headers;
        let guides = &self.inner.config.guides;
        let styles = &self.inner.config.styles;

        let log_header = match payload.metadata.level {
            Level::Error => headers.log.error.style(styles.log.error),
            Level::Warn => headers.log.warn.style(styles.log.warn),
            Level::Info => headers.log.info.style(styles.log.info),
            Level::Debug => headers.log.debug.style(styles.log.debug),
            Level::Trace => headers.log.trace.style(styles.log.trace),
        };

        write!(printer, "{prefix}{} {}", log_header, payload.message.bold())?;

        let guide = if payload.children.is_empty() {
            guides.empty
        } else {
            guides.line
        };

        let sub_prefix = format_args!("{}{}", next_prefix, guide.style(styles.guides));
        self.render_frames(printer, &payload.frames, sub_prefix)?;
        self.render_extra(printer, &payload.metadata, sub_prefix)?;
        self.render_children(printer, &payload.children, next_prefix)
    }

    fn render_extra(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        metadata: &Metadata,
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let headers = &self.inner.config.headers;
        let styles = &self.inner.config.styles;
        let mut write_location = |value: fmt::Arguments<'_>| {
            write!(
                printer,
                "{prefix}{} {}",
                headers.location.style(styles.location),
                value.style(styles.distracting)
            )
        };

        match &metadata.location {
            Location::File { name, line, column } => match (line, column) {
                (Some(line), Some(column)) => {
                    write_location(format_args!("{name}:{line}:{column}"))
                }
                (Some(line), None) => write_location(format_args!("{name}:{line}")),
                _ => write_location(format_args!("{name}")),
            },
            Location::Module(name) => write_location(format_args!("{name}")),
            Location::Unknown => Ok(()),
        }
    }

    // loops over every frame n times because sorting would require
    // allocation and we aren't going to be handling many frames anyways
    fn render_frames(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        frames: &[Frame],
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let headers = &self.inner.config.headers;
        let styles = &self.inner.config.styles;

        // suggestion pass
        for frame in frames {
            let Frame::Suggestion(suggestion) = frame else {
                continue;
            };

            write!(
                printer,
                "{prefix}{} {}",
                headers.suggestion.style(styles.suggestion),
                suggestion
            )?;
        }

        // context pass
        let mut first = true;
        for frame in frames {
            let Frame::Context { key, value } = frame else {
                continue;
            };

            if first {
                write!(printer, "{prefix}{}", headers.context.style(styles.context))?;
                first = false;
            }

            write!(
                printer,
                "{prefix}  {}",
                format_args!("{key}: {value}").style(styles.distracting)
            )?;
        }

        // attachment pass
        for frame in frames {
            let Frame::Attachment(attachment) = frame else {
                continue;
            };

            write!(
                printer,
                "{prefix}{}",
                headers.attachment.style(styles.attachment)
            )?;

            for line in attachment.lines() {
                write!(printer, "{prefix}  {}", line.style(styles.distracting))?;
            }
        }

        Ok(())
    }

    fn render_children(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        children: &[ReportPayload],
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let guides = &self.inner.config.guides;
        let styles = &self.inner.config.styles;

        let len = children.len();
        for (i, child) in children.into_iter().enumerate() {
            let last = i == len - 1;

            let guide = if last {
                guides.last_branch
            } else {
                guides.branch
            };
            let next_prefix = format_args!("{}{}", prefix, guide.style(styles.guides));

            let guide = if last { guides.empty } else { guides.line };
            let next_next_prefix = format_args!("{}{}", prefix, guide.style(styles.guides));

            self.render_report(printer, child, next_prefix, next_next_prefix)?;
        }

        Ok(())
    }
}
