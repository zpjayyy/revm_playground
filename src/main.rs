mod trace;
mod utils;

use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use anyhow::Result;
use anyhow::Ok;

pub fn setup_logger() -> Result<()> {
    let colors = ColoredLevelConfig {
        trace: Color::Cyan,
        debug: Color::Magenta,
        info: Color::Green,
        warn: Color::Red,
        error: Color::BrightRed,
        ..ColoredLevelConfig::new()
    };

    fern::Dispatch::new().format(move |out, message, record| {
        out.finish(format_args!(
            "{}[{}] {}",
            chrono::Local::now().format("[%H:%M:%S]"),
            colors.color(record.level()),
            message
        ))
    })
        .chain(std::io::stdout())
        .level(log::LevelFilter::Error)
        .level_for("revm_playground", LevelFilter::Info)
        .apply()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");
    dotenv::dotenv().ok();
    setup_logger()?;

    let weth = String::from("0x7b79995e5f793a07bc00c21412e50ecae098e7f9");
    Ok(())
}
