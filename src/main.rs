use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use yealink_fov::{Camera, Fov, WdrLevel};

enum Command {
    Fov(Fov),
    Wdr(WdrLevel),
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut serial = None;
    let mut positional = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "--serial" => {
                serial = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--serial requires a value"))?,
                );
            }
            _ if arg.starts_with('-') => bail!("unknown option {arg:?}"),
            _ => positional.push(arg),
        }
    }

    let command = match positional.as_slice() {
        [] => {
            print_help();
            bail!("missing setting");
        }
        [setting] if setting == "fov" => bail!("missing FOV"),
        [setting] if setting == "wdr" => bail!("missing WDR level"),
        [degrees] => Command::Fov(Fov::try_from(parse_value(degrees, "FOV")?)?),
        [setting, degrees] if setting == "fov" => {
            Command::Fov(Fov::try_from(parse_value(degrees, "FOV")?)?)
        }
        [setting, level] if setting == "wdr" => {
            Command::Wdr(WdrLevel::try_from(parse_value(level, "WDR level")?)?)
        }
        [setting, _] => bail!("unknown setting {setting:?}; choose fov or wdr"),
        _ => bail!("too many arguments"),
    };

    let mut camera = Camera::open(serial.as_deref())?;
    match command {
        Command::Fov(fov) => {
            camera.set_fov(fov)?;
            println!("FOV set to {} degrees", fov.degrees());
        }
        Command::Wdr(level) => {
            camera.set_wdr(level)?;
            println!("WDR level set to {}", level.value());
        }
    }
    Ok(())
}

fn parse_value(value: &str, label: &str) -> Result<i32> {
    value
        .parse::<i32>()
        .with_context(|| format!("invalid {label} {value:?}"))
}

fn print_help() {
    println!("Control a Yealink UVC30 Desktop webcam.");
    println!();
    println!("Usage:");
    println!("  yealink-fov [--serial SERIAL] <70|90|120>");
    println!("  yealink-fov [--serial SERIAL] fov <70|90|120>");
    println!("  yealink-fov [--serial SERIAL] wdr <0|1|2|3|4|5>");
}
