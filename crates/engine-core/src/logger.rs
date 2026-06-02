use std::io::{self, Write};
use std::sync::Mutex;
use std::time::SystemTime;

/// Thread-safe logger with configurable verbosity levels.
///
/// Messages below the configured level are discarded. All logged messages
/// are printed to the console and buffered for later flushing to a file.
///
/// # Example
///
/// ```rust
/// use engine_core::logger::{Logger, LogLevel};
///
/// let logger = Logger::new(LogLevel::Debug);
/// logger.info("Engine started");
/// logger.warn("Low memory");
/// logger.flush().expect("Failed to flush log");
/// ```
pub struct Logger {
    level: LogLevel,
    buffer: Mutex<Vec<String>>,
}

/// Log verbosity level, ordered from least to most verbose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LogLevel {
    /// Only errors.
    Error,
    /// Warnings and above.
    Warn,
    /// Informational messages and above (default).
    #[default]
    Info,
    /// Debug messages and above.
    Debug,
    /// All messages including trace-level detail.
    Trace,
}

impl Logger {
    /// Create a new logger that filters messages below `level`.
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            buffer: Mutex::new(Vec::new()),
        }
    }

    /// Log an error message (always printed).
    pub fn error(&self, msg: &str) {
        self.log(LogLevel::Error, msg);
    }

    /// Log a warning message.
    pub fn warn(&self, msg: &str) {
        self.log(LogLevel::Warn, msg);
    }

    /// Log an informational message.
    pub fn info(&self, msg: &str) {
        self.log(LogLevel::Info, msg);
    }

    /// Log a debug message.
    pub fn debug(&self, msg: &str) {
        self.log(LogLevel::Debug, msg);
    }

    /// Log a trace message (most verbose).
    pub fn trace(&self, msg: &str) {
        self.log(LogLevel::Trace, msg);
    }

    fn log(&self, level: LogLevel, msg: &str) {
        if level > self.level {
            return;
        }

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        let level_str = match level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        };

        let log_line = format!(
            "[{:010}.{:03}] [{}] {}",
            timestamp.as_secs(),
            timestamp.subsec_millis(),
            level_str,
            msg
        );

        // Print to console
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(log_line.clone());
        }

        match level {
            LogLevel::Error => eprintln!("{}", log_line),
            _ => println!("{}", log_line),
        }
    }

    /// Flush buffered log messages to `engine.log`.
    pub fn flush(&self) -> io::Result<()> {
        if let Ok(buffer) = self.buffer.lock() {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("engine.log")?;

            for line in buffer.iter() {
                writeln!(file, "{}", line)?;
            }
        }
        Ok(())
    }
}

// Convenience macros
#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        $logger.warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($logger:expr, $($arg:tt)*) => {
        $logger.trace(&format!($($arg)*))
    };
}
