use anyhow::{Context as _, Result};
use std::env;
use std::fmt;
use std::io::{IsTerminal as _, stderr};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{Format, FormatEvent, FormatFields, Full, Writer};
use tracing_subscriber::registry::LookupSpan;

/// Formats `INFO` events as plain messages (like `println!`) while delegating
/// all other levels to the default formatter.
struct PlainInfoFormatter {
    default: Format<Full>,
}

impl<S, N> FormatEvent<S, N> for PlainInfoFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if *event.metadata().level() == Level::INFO {
            let mut visitor = PlainMessageVisitor {
                writer: writer.by_ref(),
                result: Ok(()),
            };
            event.record(&mut visitor);
            visitor.result?;
            writeln!(writer)
        } else {
            self.default.format_event(ctx, writer, event)
        }
    }
}

/// Visits event fields and writes only the `message` field as plain text.
struct PlainMessageVisitor<'writer> {
    writer: Writer<'writer>,
    result: fmt::Result,
}

impl Visit for PlainMessageVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.result.is_ok() && field.name() == "message" {
            self.result = write!(self.writer, "{value}");
        }
    }

    #[expect(
        clippy::use_debug,
        reason = "The Visit trait provides message values as `dyn Debug`."
    )]
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_ok() && field.name() == "message" {
            self.result = write!(self.writer, "{value:?}");
        }
    }
}

/// Initialize the global tracing subscriber.
///
/// When the active log level is `INFO` (the default), `INFO` messages are printed as plain text
/// (no timestamp, level, or target). All other levels keep the default formatting. When the active
/// level is something other than `INFO`, all messages use the default formatting.
pub(crate) fn init() -> Result<()> {
    let log_level_raw = env::var("AGENTCONTAINER_LOG_LEVEL").ok();

    let (plain_info, env_filter) = log_level_raw.as_ref().map_or_else(
        || Ok::<_, anyhow::Error>((true, EnvFilter::new("info"))),
        |level| {
            let filter = EnvFilter::try_new(level)
                .with_context(|| format!("Invalid AGENTCONTAINER_LOG_LEVEL value: {level:?}"))?;
            Ok((level.eq_ignore_ascii_case("info"), filter))
        },
    )?;

    if plain_info {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .event_format(PlainInfoFormatter {
                default: Format::default().with_ansi(stderr().is_terminal()),
            })
            .with_writer(stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(stderr)
            .init();
    }

    Ok(())
}
