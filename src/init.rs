use std::io::Write;

use anyhow::Result;
use chrono::Local;
use colored::Colorize;
use env_logger::Env;
use env_logger::fmt::Color;
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
            let level_color = match record.level() {
                Level::Error => Color::Red,
                Level::Warn => Color::Yellow,
                Level::Info => Color::Green,
                Level::Debug | Level::Trace => Color::Cyan,
            };
            let mut level_style = fmt.style();
            level_style.set_color(level_color);
            writeln!(
                fmt,
                "[{} {}] {}",
                Local::now().format("%H:%M:%S%.3f"),
                level_style.value(record.level()),
                &record.args()
            )
        })
        .init();
}

pub fn print_logo() {
    let logo_1_1 = r#" ________  ________  ___   "#.green();
    let logo_1_2 = r#"|\   __  \|\   ____\|\  \  "#.green();
    let logo_1_3 = r#"\ \  \|\  \ \  \___|\ \  \ "#.green();
    let logo_1_4 = r#" \ \  \\\  \ \  \    \ \  \ "#.green();
    let logo_1_5 = r#"  \ \  \\\  \ \  \____\ \  \ "#.green();
    let logo_1_6 = r#"   \ \_______\ \_______\ \__\"#.green();
    let logo_1_7 = r#"    \|_______|\|_______|\|__|"#.green();
    let logo_2_1 = r#" ________  ________  ________  ___  __       "#.magenta();
    let logo_2_2 = r#"|\   __  \|\   __  \|\   ____\|\  \|\  \     "#.magenta();
    let logo_2_3 = r#"\ \  \|\  \ \  \|\  \ \  \___|\ \  \/  /|_   "#.magenta();
    let logo_2_4 = r#"\ \   ____\ \   __  \ \  \    \ \   ___  \  "#.magenta();
    let logo_2_5 = r#"\ \  \___|\ \  \ \  \ \  \____\ \  \\ \  \ "#.magenta();
    let logo_2_6 = r#" \ \__\    \ \__\ \__\ \_______\ \__\\ \__\"#.magenta();
    let logo_2_7 = r#"  \|__|     \|__|\|__|\|_______|\|__| \|__|"#.magenta();
    println!(
        "{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n",
        logo_1_1,
        logo_2_1,
        logo_1_2,
        logo_2_2,
        logo_1_3,
        logo_2_3,
        logo_1_4,
        logo_2_4,
        logo_1_5,
        logo_2_5,
        logo_1_6,
        logo_2_6,
        logo_1_7,
        logo_2_7,
    )
}
