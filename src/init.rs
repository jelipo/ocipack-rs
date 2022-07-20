use anyhow::Result;
use chrono::Local;
use env_logger::Env;
use std::io::Write;
use colored::Colorize;
use log::Level;

/// 整个App初始化方法
pub fn init() -> Result<()> {
    log_init();

    Ok(())
}

/// 初始化日志
fn log_init() {
    let env = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env)
        .format(|fmt, record| {
            let level = record.level();
            let level_color = match level {
                Level::Error => level.as_str().red(),
                Level::Warn => level.as_str().yellow(),
                Level::Info => level.as_str().green(),
                Level::Debug => level.as_str().red(),
                Level::Trace => level.as_str().cyan()
            };
            writeln!(
                fmt,
                "[{} {}] {}",
                Local::now().format("%H:%M:%S%.3f"),
                level_color,
                &record.args()
            )
        })
        .init();
}
