use lazy_static::lazy_static;
use regex::Regex;
use std::{
    cmp::Ordering,
    env,
    ffi::OsStr,
    fs::{read_dir, rename},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
const SEVEN_ZIP_PATHS: [&str; 4] = [
    r"7z",
    r"C:\Program Files\7-Zip\7z",
    r"C:\Program Files (x86)\7-Zip\7z",
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

fn main() -> Result<(), Error> {
    let dirs = env::args()
        .skip(1)
        .map(PathBuf::from)
        .filter(|r| r.is_dir())
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        println!("Usage: cba [dirs...]");
        println!();
        return Ok(());
    }

    let seven_zip = match SEVEN_ZIP_PATHS
        .iter()
        .find(|&&p| Command::new(p).stdout(Stdio::null()).spawn().is_ok())
    {
        Some(&c) => c,
        None => return Err(Error::from("7-zip is not installed.")),
    };

    for dir in &dirs {
        let mut images = match read_dir(&dir) {
            Ok(dir) => dir
                .filter_map(|r| r.map(|e| e.path()).ok())
                .filter(|r| !r.is_dir())
                .filter(|r| {
                    match r
                        .extension()
                        .and_then(OsStr::to_str)
                        .map(str::to_uppercase)
                        .unwrap()
                        .as_str()
                    {
                        "GIF" | "HEIC" | "JPG" | "JPEG" | "PNG" | "TIF" | "TIFF" | "WEBP" => true,
                        _ => false,
                    }
                })
                .collect::<Vec<_>>(),
            Err(e) => return Err(Error::from(e)),
        };
        if images.is_empty() {
            continue;
        }

        images.sort_by(|a, b| {
            lazy_static! {
                static ref REGEX: Regex = Regex::new(r"(\d+)").unwrap();
            }

            if let (Some(mut ai), Some(mut bi)) = (
                a.file_name()
                    .and_then(OsStr::to_str)
                    .map(|t| REGEX.captures_iter(t)),
                b.file_name()
                    .and_then(OsStr::to_str)
                    .map(|t| REGEX.captures_iter(t)),
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
                dir.join(format!(
                    "{}{}.{}",
                    "0".repeat(numbers - num.len()),
                    num,
                    ext
                ))
            })
            .collect::<Vec<_>>();

        let sources = if targets.iter().find(|&f| images.contains(f)).is_none() {
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
            format!("{}.cbz", dir.display()),
        ];

        for file in &targets {
            args.push(file.display().to_string());
        }

        Command::new(seven_zip)
            .args(args)
            .stdout(Stdio::null())
            .spawn()?;
    }

    return Ok(());
}

fn rename_all(sources: &Vec<PathBuf>, targets: &Vec<PathBuf>) -> Result<(), Error> {
    for i in 0..sources.len() {
        rename(&sources[i], &targets[i])?;
    }
    return Ok(());
}

pub type Error = Box<dyn std::error::Error>;
