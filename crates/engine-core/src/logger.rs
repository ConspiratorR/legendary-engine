use std::sync::Mutex;
use std::io::{self, Write};
use std::time::SystemTime;

pub struct Logger {
    level: LogLevel,
    buffer: Mutex<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            buffer: Mutex::new(Vec::new()),
        }
    }

    pub fn error(&self, msg: &str) {
        self.log(LogLevel::Error, msg);
    }

    pub fn warn(&self, msg: &str) {
        self.log(LogLevel::Warn, msg);
    }

    pub fn info(&self, msg: &str) {
        self.log(LogLevel::Info, msg);
    }

    pub fn debug(&self, msg: &str) {
        self.log(LogLevel::Debug, msg);
    }

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
            LogLevel::Warn  => "WARN ",
            LogLevel::Info  => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        };

        let log_line = format!("[{:010}.{:03}] [{}] {}", 
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
