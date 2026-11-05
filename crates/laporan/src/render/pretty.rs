//! Pretty rendering for [`Report`]s.

use core::fmt;

use educe::Educe;
use owo_colors::Style;

use crate::{
    Level, Location, Metadata, RawReport,
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
            distracting: Style::new().bright_black(),
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
    config: Config,
}

impl PrettyRenderer {
    /// Constructs a new `PrettyRenderer`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            config: Config::new(),
        }
    }

    /// Constructs a new `PrettyRenderer` with default settings derived from the environment.
    ///
    /// Color is enabled if `FORCE_COLOR` is set or stderr is a tty, and is disabled if `NO_COLOR` is set.
    /// Additionally, `FORCE_COLOR` has higher priority than `NO_COLOR`.
    #[cfg(feature = "std")]
    #[inline]
    #[must_use]
    pub fn from_env() -> Self {
        use std::io::IsTerminal;

        let exists = |name: &str| std::env::var_os(name).is_some();
        let color = if exists("FORCE_COLOR") {
            true
        } else if exists("NO_COLOR") {
            false
        } else {
            std::io::stderr().is_terminal()
        };

        Self {
            config: Config {
                color,
                ..Default::default()
            },
        }
    }

    /// Replaces the underlying render configuration for this renderer.
    #[inline]
    #[must_use]
    pub const fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
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

impl Renderer for PrettyRenderer {
    #[inline]
    fn render<'a>(&'a self, raw: &'a RawReport) -> impl fmt::Display + 'a {
        PrettyDisplayer { inner: self, raw }
    }
}

struct LinePrinter<'a, 'b> {
    fmt: &'a mut fmt::Formatter<'b>,
    first: bool,
}

impl LinePrinter<'_, '_> {
    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        if !self.first {
            self.fmt.write_str("\n")?;
        }

        self.first = false;
        self.fmt.write_fmt(args)
    }
}

#[derive(Debug, Copy, Clone)]
struct PrettyDisplayer<'a> {
    inner: &'a PrettyRenderer,
    raw: &'a RawReport,
}

impl fmt::Display for PrettyDisplayer<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut printer = LinePrinter {
            fmt: f,
            first: true,
        };

        self.render_report(&mut printer, self.raw, format_args!(""), format_args!(""))
    }
}

impl PrettyDisplayer<'_> {
    #[inline]
    fn render_value<T: fmt::Display>(&self, value: T, style: Style) -> MaybeStyled<T> {
        if self.inner.config.color {
            MaybeStyled::Styled(style.style(value))
        } else {
            MaybeStyled::Plain(value)
        }
    }

    #[inline]
    fn with_fallback<'a>(&self, text: &'a str, fallback: &'static str) -> &'a str {
        if text.is_empty() { fallback } else { text }
    }

    fn write_block(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        prefix: fmt::Arguments<'_>,
        first_prefix: fmt::Arguments<'_>,
        content: &str,
        style: Style,
    ) -> fmt::Result {
        let mut lines = content.lines();

        write!(
            printer,
            "{prefix}{first_prefix}{}",
            self.render_value(lines.next().unwrap_or(""), style)
        )?;

        lines.into_iter().try_for_each(|line| {
            let line = self.render_value(line, style);
            write!(printer, "{prefix}  {line}")
        })
    }

    fn render_report(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        raw: &RawReport,
        prefix: fmt::Arguments<'_>,
        next_prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let log_headers = &self.inner.config.headers.log;
        let guides = &self.inner.config.guides;
        let styles = &self.inner.config.styles;

        let log_header = match raw.metadata.level {
            Level::Error => self.render_value(log_headers.error, styles.log.error),
            Level::Warn => self.render_value(log_headers.warn, styles.log.warn),
            Level::Info => self.render_value(log_headers.info, styles.log.info),
            Level::Debug => self.render_value(log_headers.debug, styles.log.debug),
            Level::Trace => self.render_value(log_headers.trace, styles.log.trace),
        };

        let message = self.with_fallback(&raw.message, "no message");
        self.write_block(
            printer,
            prefix,
            format_args!("{} ", log_header),
            message,
            Style::new().bold(),
        )?;

        let guide = if raw.children.is_empty() {
            guides.empty
        } else {
            guides.line
        };

        let sub_prefix = format_args!("{}{}", next_prefix, self.render_value(guide, styles.guides));
        self.render_frames(printer, raw, sub_prefix)?;
        self.render_extra(printer, &raw.metadata, sub_prefix)?;
        self.render_children(printer, &raw.children, next_prefix)
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
                self.render_value(headers.location, styles.location),
                self.render_value(value, styles.distracting)
            )
        };

        match &metadata.location {
            Location::File {
                name,
                line: Some(line),
                column: Some(column),
            } => write_location(format_args!("{name}:{line}:{column}")),
            Location::File {
                name,
                line: Some(line),
                column: None,
            } => write_location(format_args!("{name}:{line}")),
            Location::Module(name) | Location::File { name, .. } => {
                write_location(format_args!("{name}"))
            }
            Location::Unknown => Ok(()),
        }
    }

    // loops over every frame n times because sorting would require
    // allocation and we aren't going to be handling many frames anyways
    fn render_frames(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        raw: &RawReport,
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let headers = &self.inner.config.headers;
        let styles = &self.inner.config.styles;

        // suggestion pass
        for Suggestion { content } in raw.frames_of::<Suggestion>() {
            self.write_block(
                printer,
                prefix,
                format_args!(
                    "{} ",
                    self.render_value(headers.suggestion, styles.suggestion)
                ),
                self.with_fallback(content, "no suggestion"),
                Style::new(),
            )?;
        }

        // context pass
        raw.frames_of::<Context>()
            .try_fold(true, |first, Context { key, value }| {
                if first {
                    write!(
                        printer,
                        "{prefix}{}",
                        self.render_value(headers.context, styles.context)
                    )?;
                }

                write!(
                    printer,
                    "{prefix}  {}: {}",
                    self.render_value(self.with_fallback(key, "no key"), styles.distracting),
                    self.render_value(self.with_fallback(value, "no value"), styles.distracting)
                )?;

                Ok(false)
            })?;

        // attachment pass
        for Attachment { content } in raw.frames_of::<Attachment>() {
            write!(
                printer,
                "{prefix}{}",
                self.render_value(headers.attachment, styles.attachment)
            )?;

            for line in self.with_fallback(content, "no content").lines() {
                write!(
                    printer,
                    "{prefix}  {}",
                    self.render_value(line, styles.distracting)
                )?;
            }
        }

        Ok(())
    }

    fn render_children(
        &self,
        printer: &mut LinePrinter<'_, '_>,
        children: &[RawReport],
        prefix: fmt::Arguments<'_>,
    ) -> fmt::Result {
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
            let next_prefix = format_args!("{}{}", prefix, self.render_value(guide, styles.guides));

            let guide = if last { guides.empty } else { guides.line };
            let next_next_prefix =
                format_args!("{}{}", prefix, self.render_value(guide, styles.guides));

            self.render_report(printer, child, next_prefix, next_next_prefix)?;
        }

        Ok(())
    }
}
