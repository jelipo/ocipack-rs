use anyhow::Result;
use chrono::Local;
use env_logger::Env;
use std::io::Write;

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
            writeln!(
                fmt,
                "[{} {}] {}",
                Local::now().format("%H:%M:%S%.3f"),
                record.level(),
                &record.args()
            )
        })
        .init();
}
