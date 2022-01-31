use anyhow::Result;
use env_logger::Env;

/// 整个App初始化方法
pub fn init() -> Result<()> {
    log_init();

    Ok(())
}

/// 初始化日志
fn log_init() {
    let env = Env::default()
        .default_filter_or("info");
    env_logger::init_from_env(env);
}
