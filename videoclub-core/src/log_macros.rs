/// Macros de log que usan `println!` internamente.
///
/// Reimplementan los macros del crate `log` para evitar la dependencia de
/// `env_logger`, que no funciona correctamente con GTK4. La salida se escribe
/// a stdout con formato `[HH:MM:SS LEVEL] mensaje`.
///
/// El timestamp usa `std::time::SystemTime`, sin dependencias externas.

/// Macro interna para formatear timestamp HH:MM:SS (UTC).
#[doc(hidden)]
#[macro_export]
macro_rules! _log_timestamp {
    () => {{
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        println!("[{} DEBUG] {}", $crate::_log_timestamp!(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        println!("[{} INFO] {}", $crate::_log_timestamp!(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        println!("[{} WARN] {}", $crate::_log_timestamp!(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        println!("[{} ERROR] {}", $crate::_log_timestamp!(), format!($($arg)*))
    };
}
