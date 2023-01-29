use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{read_dir, rename},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
const SEVEN_ZIP_PATHS: [&str; 3] = [
    r"7z.exe",
    r"C:\Program Files\7-Zip\7z.exe",
    r"C:\Program Files (x86)\7-Zip\7z.exe",
];

#[cfg(not(target_os = "windows"))]
const SEVEN_ZIP_PATHS: [&str; 8] = [
    r"7zz",
    r"/usr/local/bin/7zz",
    r"/opt/local/bin/7zz",
    r"/opt/homebrew/bin/7zz",
    r"7z",
    r"/usr/local/bin/7z",
    r"/opt/local/bin/7z",
    r"/opt/homebrew/bin/7z",
];

#[derive(Parser)]
struct Args {
    #[arg(required = true)]
    dirs: Vec<PathBuf>,
}

fn main() -> Result<(), Error> {
    #[cfg(target_os = "windows")]
    unsafe {
        use winapi::um::{wincon::SetConsoleOutputCP, winnls::CP_UTF8};
        SetConsoleOutputCP(CP_UTF8);
    }

    let args = Args::parse();

    let seven_zip = match SEVEN_ZIP_PATHS
        .iter()
        .find(|&&p| Command::new(p).stdout(Stdio::null()).spawn().is_ok())
    {
        Some(&c) => c,
        None => return Err(Error::from("7-zip is not installed.")),
    };

    for path in &args.dirs {
        let mut images = match read_dir(path) {
            Ok(dir) => dir
                .filter_map(|r| r.map(|d| d.path()).ok())
                .filter(|p| !p.is_dir())
                .filter(|p| {
                    match p
                        .extension()
                        .and_then(OsStr::to_str)
                        .map(str::to_uppercase)
                        .unwrap()
                        .as_str()
                    {
                        "AVIF" | "GIF" | "HEIC" | "JPG" | "JPEG" | "PNG" | "TIF" | "TIFF"
                        | "WEBP" => true,
                        _ => false,
                    }
                })
                .collect::<Vec<_>>(),
            Err(e) => return Err(Error::from(e)),
        };
        if images.is_empty() {
            continue;
        }

        lazy_static! {
            static ref NUMBER_REGEX: Regex = Regex::new(r"(\d+)").unwrap();
        }
        images.sort_by(|a, b| {
            if let (Some(mut ai), Some(mut bi)) = (
                a.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| NUMBER_REGEX.captures_iter(n)),
                b.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| NUMBER_REGEX.captures_iter(n)),
            ) {
                loop {
                    if let (Some(an), Some(bn)) = (
                        ai.next().and_then(|c| c[1].parse::<u64>().ok()),
                        bi.next().and_then(|c| c[1].parse::<u64>().ok()),
                    ) {
                        let cmp = an.cmp(&bn);
                        if cmp != Ordering::Equal {
                            return cmp;
                        }
                        continue;
                    }
                    break;
                }
            }

            return a.cmp(b);
        });

        let numbers = f64::ceil(f64::log10(images.len() as f64)) as usize;
        let targets = (0..images.len())
            .map(|i| {
                let num = i.to_string();
                let ext = match images[i]
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(str::to_uppercase)
                    .unwrap()
                    .as_str()
                {
                    "JPEG" => "JPG".to_owned(),
                    s => s.to_owned(),
                };
                path.join(format!(
                    "{}{}.{}",
                    "0".repeat(numbers - num.len()),
                    num,
                    ext
                ))
            })
            .collect::<Vec<_>>();

        let sources = if targets.iter().find(|&p| images.contains(p)).is_none() {
            images
        } else {
            let targets = images
                .iter()
                .map(|b| {
                    PathBuf::from(format!(
                        "{}/_{}",
                        b.parent().and_then(Path::to_str).unwrap(),
                        b.file_name().and_then(OsStr::to_str).unwrap(),
                    ))
                })
                .collect();
            rename_all(&images, &targets)?;
            targets
        };

        rename_all(&sources, &targets)?;

        let mut args = vec![
            "a".to_owned(),
            "-bd".to_owned(),
            "-tzip".to_owned(),
            "-mx=9".to_owned(),
            "-mfb=258".to_owned(),
            "-scsUTF-8".to_owned(),
            format!("{}.cbz", path.display()),
        ];

        for file in &targets {
            args.push(file.display().to_string());
        }

        Command::new(seven_zip)
            .args(args)
            .stdout(Stdio::null())
            .spawn()?
            .wait()?;
    }

    return Ok(());
}

fn rename_all(sources: &Vec<PathBuf>, targets: &Vec<PathBuf>) -> Result<(), Error> {
    for i in 0..usize::min(sources.len(), targets.len()) {
        rename(&sources[i], &targets[i])?;
    }
    return Ok(());
}

pub type Error = Box<dyn std::error::Error>;
