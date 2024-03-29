use clap::Parser;
use lazy_regex::{lazy_regex, Lazy, Regex};
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{read_dir, rename},
    path::PathBuf,
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
const SEVEN_ZIP_PATHS: [&str; 3] = [
    r"7z.exe",
    r"C:\Program Files\7-Zip\7z.exe",
    r"C:\Program Files (x86)\7-Zip\7z.exe",
];

#[cfg(not(target_os = "windows"))]
const SEVEN_ZIP_PATHS: [&str; 14] = [
    r"7zz",
    r"/usr/bin/7zz",
    r"/usr/local/bin/7zz",
    r"/opt/local/bin/7zz",
    r"/opt/homebrew/bin/7zz",
    r"7z",
    r"/usr/bin/7z",
    r"/usr/local/bin/7z",
    r"/opt/local/bin/7z",
    r"/opt/homebrew/bin/7z",
    r"7zr",
    r"/usr/bin/7zr",
    r"/usr/local/bin/7zr",
    r"/opt/local/bin/7zr",
];

const EXTENSIONS: [&str; 9] = [
    "AVIF", "GIF", "HEIC", "JPG", "JPEG", "PNG", "TIF", "TIFF", "WEBP",
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

    let seven_zip = *SEVEN_ZIP_PATHS
        .iter()
        .find(|&&p| Command::new(p).stdout(Stdio::null()).spawn().is_ok())
        .ok_or("7-zip is not installed.")?;

    for path in &args.dirs {
        let mut images = vec![];
        for dir in read_dir(path)? {
            let path = dir?.path();
            if !path.is_dir()
                && path
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(str::to_uppercase)
                    .filter(|s| EXTENSIONS.contains(&s.as_str()))
                    .is_some()
            {
                images.push(path);
            }
        }
        if images.is_empty() {
            continue;
        }

        images.sort_by(|a, b| {
            static REGEX: Lazy<Regex> = lazy_regex!(r"(\d+)");

            if let (Some(mut ai), Some(mut bi)) = (
                a.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| REGEX.captures_iter(n)),
                b.file_name()
                    .and_then(OsStr::to_str)
                    .map(|n| REGEX.captures_iter(n)),
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

        let names = digit(images.len());
        let targets = (0..images.len())
            .filter_map(|i| {
                let ext = images[i]
                    .extension()
                    .and_then(OsStr::to_str)?
                    .to_uppercase();
                let ext = match ext.as_str() {
                    "JPEG" => "JPG".to_owned(),
                    _ => ext,
                };
                let image = path.join(format!("{}{}.{}", "0".repeat(names - digit(i)), i, ext));
                return Some(image);
            })
            .collect::<Vec<_>>();

        let sources = if targets.iter().find(|&p| images.contains(p)).is_none() {
            images
        } else {
            let parent = path.display();
            let targets = images
                .iter()
                .filter_map(|b| {
                    let child = b.file_name().and_then(OsStr::to_str)?;
                    let path = PathBuf::from(format!("{}/_{}", parent, child));
                    return Some(path);
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

fn digit(i: usize) -> usize {
    return (i as f64).log10().floor() as usize + 1;
}

pub type Error = Box<dyn std::error::Error>;
