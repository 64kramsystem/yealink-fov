use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use yealink_fov::{Camera, Fov};

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
    let mut degrees = None;

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
            _ if degrees.is_none() => {
                degrees = Some(
                    arg.parse::<i32>()
                        .with_context(|| format!("invalid FOV {arg:?}"))?,
                );
            }
            _ => bail!("unexpected argument {arg:?}"),
        }
    }

    let Some(degrees) = degrees else {
        print_help();
        bail!("missing FOV");
    };
    let fov = Fov::try_from(degrees)?;
    let mut camera = Camera::open(serial.as_deref())?;
    camera.set_fov(fov)?;
    println!("FOV set to {} degrees", fov.degrees());
    Ok(())
}

fn print_help() {
    println!("Set the field of view on a Yealink UVC30 Desktop webcam.");
    println!();
    println!("Usage: yealink-fov [--serial SERIAL] <70|90|120>");
}
