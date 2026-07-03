use std::backtrace::Backtrace;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_IDENTIFIER: &str = "pr-monitor.guibeira.dev";

static PANIC_HOOK: Once = Once::new();

pub fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let default_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |panic_info| {
            let current_thread = std::thread::current();
            let thread_name = current_thread.name().unwrap_or("<unnamed>");
            let payload = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
                (*message).to_owned()
            } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
                message.clone()
            } else {
                "<non-string panic payload>".to_owned()
            };

            let mut details = String::new();
            let _ = writeln!(details, "timestamp_unix: {}", timestamp_unix());
            let _ = writeln!(details, "version: {}", env!("CARGO_PKG_VERSION"));
            let _ = writeln!(details, "pid: {}", std::process::id());
            let _ = writeln!(details, "thread: {thread_name}");
            let _ = writeln!(details, "payload: {payload}");

            if let Some(location) = panic_info.location() {
                let _ = writeln!(
                    details,
                    "location: {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }

            let _ = writeln!(details, "backtrace:\n{}", Backtrace::force_capture());

            append_diagnostic("panic.log", &details);
            default_hook(panic_info);
        }));
    });
}

pub fn record_fatal_error(context: &str, err: &impl std::fmt::Display) {
    let mut details = String::new();
    let _ = writeln!(details, "timestamp_unix: {}", timestamp_unix());
    let _ = writeln!(details, "version: {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(details, "pid: {}", std::process::id());
    let _ = writeln!(details, "context: {context}");
    let _ = writeln!(details, "error: {err}");
    let _ = writeln!(details, "backtrace:\n{}", Backtrace::force_capture());

    append_diagnostic("fatal.log", &details);
}

pub fn app_log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join(APP_IDENTIFIER);
    }

    std::env::temp_dir().join(APP_IDENTIFIER).join("logs")
}

fn append_diagnostic(file_name: &str, details: &str) {
    let log_dir = app_log_dir();
    if let Err(err) = std::fs::create_dir_all(&log_dir) {
        eprintln!("failed to create diagnostic log directory {log_dir:?}: {err}");
        return;
    }

    let log_file = log_dir.join(file_name);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "==== diagnostic event ====");
            let _ = file.write_all(details.as_bytes());
            let _ = writeln!(file);
        }
        Err(err) => eprintln!("failed to open diagnostic log file {log_file:?}: {err}"),
    }
}

fn timestamp_unix() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:03}", duration.as_secs(), duration.subsec_millis()),
        Err(_) => "before-unix-epoch".to_owned(),
    }
}
