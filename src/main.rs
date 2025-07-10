use anyhow::Result;

mod application;
mod graphics;
mod localization;
mod platform;
mod simulation;
mod storage;

fn main() -> Result<()> {
    application::execute()
}
