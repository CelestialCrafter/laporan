//! Pretty rendering for [`Report`]s.

use std::fmt;

use educe::Educe;
use owo_colors::Style;

use crate::{
    Level, Location, Metadata, ReportPayload,
    frame::{Attachment, Context, Suggestion},
    render::Renderer,
};

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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
#[derive(Debug, Copy, Clone)]
#[expect(missing_docs)]
pub struct Config {
    pub guides: Guides,
    pub headers: Headers,
    pub styles: Styles,
    pub color: bool,
}

impl Config {
    /// Constructs a new `Config`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            guides: Guides::new(),
            headers: Headers::new(),
            styles: Styles::new(),
            color: true,
        }
    }
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Pretty renderer for [`Report`]s.
///
/// [`Report`]s can be rendered via the [`Renderer`] trait.
#[derive(Default, Debug, Copy, Clone)]
pub struct PrettyRenderer {
    /// Configuration for this renderer
    config: Config,
}

impl PrettyRenderer {
    /// Constructs a new `PrettyRenderer`.
    ///
    /// The default value for `Config::color` depends on `FORCE_COLOR` and `NO_COLOR`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let mut config = Config::new();
        config.color = colors_enabled_from_env();
        owo_colors::set_override(config.color);
        Self { config }
    }

    /// Replaces the underlying render configuration for this renderer.
    #[inline]
    #[must_use]
    pub const fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }
}

// FIX: remove/rework this once no_std environments are supported
fn colors_enabled_from_env() -> bool {
    let set_non_empty = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());

    if set_non_empty("FORCE_COLOR") {
        true
    } else {
        !set_non_empty("NO_COLOR")
    }
}

enum MaybeStyled<T> {
    Styled(owo_colors::Styled<T>),
    Plain(T),
}

impl<T: fmt::Display> fmt::Display for MaybeStyled<T> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Styled(value) => value.fmt(formatter),
            Self::Plain(value) => value.fmt(formatter),
        }
    }
}

#[inline]
fn paint<T: fmt::Display>(enabled: bool, value: T, style: Style) -> MaybeStyled<T> {
    if enabled {
        MaybeStyled::Styled(style.style(value))
    } else {
        MaybeStyled::Plain(value)
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

impl LinePrinter<'_, '_> {
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
        let color = self.inner.config.color;
        let log_headers = &self.inner.config.headers.log;
        let guides = &self.inner.config.guides;
        let styles = &self.inner.config.styles;

        let log_header = match payload.metadata.level {
            Level::Error => paint(color, log_headers.error, styles.log.error),
            Level::Warn => paint(color, log_headers.warn, styles.log.warn),
            Level::Info => paint(color, log_headers.info, styles.log.info),
            Level::Debug => paint(color, log_headers.debug, styles.log.debug),
            Level::Trace => paint(color, log_headers.trace, styles.log.trace),
        };

        let message = if payload.message.is_empty() {
            "no message"
        } else {
            payload.message.as_str()
        };

        let message_style = Style::new().bold();
        let mut message_lines = message.lines();
        let first = message_lines.next().unwrap_or("");
        write!(
            printer,
            "{prefix}{} {}",
            log_header,
            paint(color, first, message_style)
        )?;
        for line in message_lines {
            write!(printer, "{prefix}  {}", paint(color, line, message_style))?;
        }

        let guide = if payload.children.is_empty() {
            guides.empty
        } else {
            guides.line
        };

        let sub_prefix = format_args!("{}{}", next_prefix, paint(color, guide, styles.guides));
        self.render_frames(printer, payload, sub_prefix)?;
        self.render_extra(printer, &payload.metadata, sub_prefix)?;
        self.render_children(printer, &payload.children, next_prefix)
    }

    fn render_extra(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        metadata: &Metadata,
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let color = self.inner.config.color;
        let headers = &self.inner.config.headers;
        let styles = &self.inner.config.styles;
        let mut write_location = |value: fmt::Arguments<'_>| {
            write!(
                printer,
                "{prefix}{} {}",
                paint(color, headers.location, styles.location),
                paint(color, value, styles.distracting)
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
        payload: &ReportPayload,
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let color = self.inner.config.color;
        let headers = &self.inner.config.headers;
        let styles = &self.inner.config.styles;

        // suggestion pass
        for Suggestion { content } in payload.frames_of::<Suggestion>() {
            let content = if content.is_empty() {
                "no suggestion"
            } else {
                content.as_str()
            };
            let mut lines = content.lines();
            let first = lines.next().unwrap_or("");
            write!(
                printer,
                "{prefix}{} {}",
                paint(color, headers.suggestion, styles.suggestion),
                first
            )?;
            for line in lines {
                write!(printer, "{prefix}  {line}")?;
            }
        }

        // context pass
        let mut first_context = true;
        for Context { key, value } in payload.frames_of::<Context>() {
            if first_context {
                write!(
                    printer,
                    "{prefix}{}",
                    paint(color, headers.context, styles.context)
                )?;
                first_context = false;
            }

            let key = if key.is_empty() {
                "no key"
            } else {
                key.as_str()
            };
            let value = if value.is_empty() {
                "no value"
            } else {
                value.as_str()
            };
            let mut lines = value.lines();
            let first = lines.next().unwrap_or("");
            write!(
                printer,
                "{prefix}  {}: {}",
                paint(color, key, styles.distracting),
                paint(color, first, styles.distracting)
            )?;
            for line in lines {
                write!(
                    printer,
                    "{prefix}    {}",
                    paint(color, line, styles.distracting)
                )?;
            }
        }

        // attachment pass
        for Attachment { content } in payload.frames_of::<Attachment>() {
            write!(
                printer,
                "{prefix}{}",
                paint(color, headers.attachment, styles.attachment)
            )?;

            let content = if content.is_empty() {
                "no content"
            } else {
                content.as_str()
            };
            for line in content.lines() {
                write!(
                    printer,
                    "{prefix}  {}",
                    paint(color, line, styles.distracting)
                )?;
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
        let color = self.inner.config.color;
        let guides = &self.inner.config.guides;
        let styles = &self.inner.config.styles;

        let len = children.len();
        for (i, child) in children.iter().enumerate() {
            let last = i == len - 1;

            let guide = if last {
                guides.last_branch
            } else {
                guides.branch
            };
            let next_prefix = format_args!("{}{}", prefix, paint(color, guide, styles.guides));

            let guide = if last { guides.empty } else { guides.line };
            let next_next_prefix = format_args!("{}{}", prefix, paint(color, guide, styles.guides));

            self.render_report(printer, child, next_prefix, next_next_prefix)?;
        }

        Ok(())
    }
}
