use std::time::Duration;

/// Formats a duration as MM:SS or HH:MM:SS
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Formats a duration in a compact form (e.g., "3:45" instead of "03:45")
pub fn format_duration_compact(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

/// Represents formatted progress information ready for display
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedProgress {
    pub elapsed_text: String,
    pub total_text: String,
    pub combined_label: String,
    pub percentage: u8,
}

/// Strategy pattern for different progress label formats
pub trait ProgressLabelFormatter {
    fn format(&self, elapsed: Duration, total: Duration, percentage: u8) -> String;
}

/// Default formatter: "00:34 / 03:10"
pub struct DefaultProgressFormatter;

impl ProgressLabelFormatter for DefaultProgressFormatter {
    fn format(&self, elapsed: Duration, total: Duration, _percentage: u8) -> String {
        format!(
            "{} / {}",
            format_duration(elapsed),
            format_duration(total)
        )
    }
}

/// Compact formatter: "0:34/3:10"
pub struct CompactProgressFormatter;

impl ProgressLabelFormatter for CompactProgressFormatter {
    fn format(&self, elapsed: Duration, total: Duration, _percentage: u8) -> String {
        format!(
            "{}/{}",
            format_duration_compact(elapsed),
            format_duration_compact(total)
        )
    }
}

/// Percentage formatter: "18% (0:34/3:10)"
pub struct PercentageProgressFormatter;

impl ProgressLabelFormatter for PercentageProgressFormatter {
    fn format(&self, elapsed: Duration, total: Duration, percentage: u8) -> String {
        format!(
            "{}% ({}/{})",
            percentage,
            format_duration_compact(elapsed),
            format_duration_compact(total)
        )
    }
}

/// Factory for creating formatted progress information
pub struct ProgressFormatter<F: ProgressLabelFormatter> {
    label_formatter: F,
}

impl<F: ProgressLabelFormatter> ProgressFormatter<F> {
    pub fn new(label_formatter: F) -> Self {
        Self { label_formatter }
    }

    pub fn format(&self, elapsed: Duration, total: Duration, percentage: u8) -> FormattedProgress {
        FormattedProgress {
            elapsed_text: format_duration(elapsed),
            total_text: format_duration(total),
            combined_label: self.label_formatter.format(elapsed, total, percentage),
            percentage,
        }
    }
}

// Convenience constructors
impl ProgressFormatter<DefaultProgressFormatter> {
    pub fn default_formatter() -> Self {
        Self::new(DefaultProgressFormatter)
    }
}

impl ProgressFormatter<CompactProgressFormatter> {
    pub fn compact_formatter() -> Self {
        Self::new(CompactProgressFormatter)
    }
}

impl ProgressFormatter<PercentageProgressFormatter> {
    pub fn percentage_formatter() -> Self {
        Self::new(PercentageProgressFormatter)
    }
}