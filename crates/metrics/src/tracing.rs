// tracing.rs — Structured logging setup.
//
// We provide a simple log macro set and an init function that configures
// the log level from the node config. In production, replace with the
// `tracing` + `tracing-subscriber` crates for structured JSON logging.
//
// For now we use Rust's built-in `eprintln!` with level prefixes —
// zero dependencies, works everywhere including Termux/Android.

/// Log levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn  = 1,
    Info  = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn"  => LogLevel::Warn,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _       => LogLevel::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn  => "WARN ",
            LogLevel::Info  => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

/// Global log level (set once at startup).
static mut CURRENT_LEVEL: LogLevel = LogLevel::Info;

/// Initialise the logging system with the given level string.
///
/// Call once during node startup before any logging occurs.
pub fn init_tracing(level: &str) {
    let parsed = LogLevel::from_str(level);
    // Safe: called once at startup before any threads are spawned.
    unsafe { CURRENT_LEVEL = parsed; }
    log(LogLevel::Info, "metrics", &format!("Log level set to: {}", level));
}

/// Current effective log level.
pub fn current_level() -> LogLevel {
    unsafe { CURRENT_LEVEL }
}

/// Log a message at the given level.
///
/// Format: `[LEVEL] <module>: <message>`
pub fn log(level: LogLevel, module: &str, msg: &str) {
    if level <= current_level() {
        let ts = primitives::Timestamp::now().as_millis();
        eprintln!("[{}] [{}ms] {}: {}", level.as_str(), ts, module, msg);
    }
}

/// Convenience macros for each log level.
#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::tracing::log($crate::tracing::LogLevel::Error, $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::tracing::log($crate::tracing::LogLevel::Warn, $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::tracing::log($crate::tracing::LogLevel::Info, $module, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::tracing::log($crate::tracing::LogLevel::Debug, $module, &format!($($arg)*))
    };
}
