use crossbeam_channel::{unbounded, Sender as CrossbeamSender, RecvTimeoutError};
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::sync::Mutex;
use std::time::Duration;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write, BufReader};
use std::sync::Arc;
use std::io::BufRead;

use crate::default::{DEBUG_COLOR, ERROR_COLOR, INFO_COLOR, WARNING_COLOR};
use crate::utils::get_current_time;
/*
# 日志模块
## 用法
### 初始化
Logger::init()
.debug(true)
.record(true)
.roll(1000)
.color(true)
.time_zone("Asia/Shanghai")
.build();   <-- 用于启动日志模块，不可忽略
使用宏记录日志
debug!() info!() warning!() error!()
退出前，使用 quit! 宏来安全退出
quit!()
 */



// 默认日志文件名
const FILE_PATH: &str = "LM.log";

#[derive(Debug)]
struct LoggerConfig {
    debug: bool,
    record: bool,
    roll: u64,
    color: bool,
    time_zone: String,
}

#[derive(Debug)]
pub struct Logger {
    config: Arc<LoggerConfig>,
    sender: Option<CrossbeamSender<LogMessage>>,
}

#[derive(Debug)]
enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug)]
enum LogMessage {
    Log(String),
    Quit,
}

pub static LOGGER: OnceLock<Logger> = OnceLock::new();

static THREAD_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

pub struct LoggerBuilder {
    debug: bool,
    record: bool,
    roll: u64,
    color: bool,
    time_zone: String,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        LoggerBuilder {
            debug: false,
            record: false,
            roll: 0,
            color: false,
            time_zone: String::from("Asia/Shanghai"),
        }
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn record(mut self, record: bool) -> Self {
        self.record = record;
        self
    }

    pub fn roll(mut self, roll: u64) -> Self {
        self.roll = roll;
        self
    }

    pub fn color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }

    pub fn time_zone(mut self, time_zone: &str) -> Self {
        self.time_zone = time_zone.to_string();
        self
    }

    pub fn build(self) {
        let config = Arc::new(LoggerConfig {
            debug: self.debug,
            record: self.record,
            roll: self.roll,
            color: self.color,
            time_zone: self.time_zone,
        });

        let sender = if self.record {
            let (sender, receiver) = unbounded();
            let roll = self.roll;
            let file_path = FILE_PATH;
            let handle = std::thread::spawn(move || {
                let mut writer = BufWriter::new(
                    OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(true)
                        .open(file_path)
                        .unwrap(),
                );
                let mut counter = 0;
                loop {
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(LogMessage::Log(message)) => {
                            // 写入日志消息
                            writeln!(writer, "{}", message).unwrap();
                            
                            // 滚动检查
                            if roll > 0 {
                                counter += 1;
                                if counter % 100 == 0 {
                                    // 强制刷新writer
                                    drop(writer);
                                    
                                    // 读取文件内容
                                    let file = OpenOptions::new()
                                        .read(true)
                                        .open(file_path)
                                        .unwrap();
                                    let reader = BufReader::new(file);
                                    let lines: Vec<String> = reader.lines()
                                        .filter_map(Result::ok)
                                        .collect();
                                    
                                    // 如果超过最大行数，只保留最新的roll行
                                    if lines.len() as u64 > roll {
                                        let start = lines.len() - roll as usize;
                                        let trimmed = lines[start..].join("\n") + "\n";
                                        std::fs::write(file_path, trimmed).unwrap();
                                    }
                                    
                                    // 重新打开文件进行追加写入
                                    writer = BufWriter::new(
                                        OpenOptions::new()
                                            .create(true)
                                            .write(true)
                                            .append(true)
                                            .open(file_path)
                                            .unwrap(),
                                    );
                                }
                            }
                            
                            writer.flush().unwrap();
                        }
                        Ok(LogMessage::Quit) => {
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            writer.flush().unwrap();
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
                writer.flush().unwrap();
            });
            *THREAD_HANDLE.lock().unwrap() = Some(handle);
            Some(sender)
        } else {
            None
        };

        let logger = Logger {
            config,
            sender,
        };
        LOGGER.set(logger).unwrap();
    }
}

impl Logger {
    /// 初始化 Logger
    pub fn init() -> LoggerBuilder {
        LoggerBuilder::new()
    }

    fn log(&self, level: LogLevel, message: &str) {
        let should_log = match level {
            LogLevel::Debug => self.config.debug,
            _ => true,
        };
        if !should_log {
            return;
        }

        let (display_color, end_color) = if self.config.color {
            let color = match level {
                LogLevel::Debug => *DEBUG_COLOR,
                LogLevel::Info => *INFO_COLOR,
                LogLevel::Warning => *WARNING_COLOR,
                LogLevel::Error => *ERROR_COLOR,
            };
            (color, "\x1b[0m")
        } else {
            ("", "")
        };

        let display_level = match level {
            LogLevel::Debug => "调试",
            LogLevel::Info => "信息",
            LogLevel::Warning => "警告",
            LogLevel::Error => "错误",
        };

        let time = get_current_time(&self.config.time_zone);
        if self.config.color {
            // 颜色渲染
            let display_message = format!(
                "{} {}[{}] {}{}",
                time, display_color, display_level, message, end_color
            );
            println!("{}", display_message);
        } else {
            let display_message = format!(
                "{} [{}] {}",
                time, display_level, message
            );
            println!("{}", display_message);
        }

        // Log formatting and printing logic...
        if self.config.record {
            if let Some(sender) = &self.sender {
                let log_line = format!("|{}|{}|{}", time, display_level, message);
                let _ = sender.send(LogMessage::Log(log_line));
            }
        }
    }

    pub fn info(&self, message: &str) { self.log(LogLevel::Info, message); }

    pub fn debug(&self, message: &str) { self.log(LogLevel::Debug, message); }

    pub fn warning(&self, message: &str) { self.log(LogLevel::Warning, message); }

    pub fn error(&self, message: &str) { self.log(LogLevel::Error, message); }
    
pub fn quit() {
    if let Some(logger) = LOGGER.get() {
        if let Some(sender) = &logger.sender {
            let _ = sender.send(LogMessage::Quit);
        }
    }
    if let Some(handle) = THREAD_HANDLE.lock().unwrap().take() {
        let _ = handle.join();
    }
}
}

#[macro_export]
macro_rules! debug {
    ($msg:expr) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.debug($msg);
        }
    };
    ($($arg:tt)*) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.debug(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! info {
    ($msg:expr) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.info($msg);
        }
    };
    ($($arg:tt)*) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.info(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($msg:expr) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.warning($msg);
        }
    };
    ($($arg:tt)*) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.warning(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.error($msg);
        }
    };
    ($($arg:tt)*) => {
        if let Some(logger) = crate::log::LOGGER.get() {
            logger.error(&format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! quit {
    () => {
        crate::log::Logger::quit();
    };
}
