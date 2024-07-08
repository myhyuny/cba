use clap::Parser;
use lazy_regex::{lazy_regex, Lazy, Regex};
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{read_dir, rename, File},
    io::{copy, Write},
    path::PathBuf,
};
use zip::{
    write::SimpleFileOptions,
    CompressionMethod::{self, Deflated, Stored},
    ZipWriter,
};

struct Extension<'n> {
    name: &'n str,
    method: CompressionMethod,
    level: Option<i64>,
}

const EXTENSIONS: [Extension; 6] = [
    Extension {
        name: "AVIF",
        method: Stored,
        level: None,
    },
    Extension {
        name: "GIF",
        method: Deflated,
        level: Some(9),
    },
    Extension {
        name: "JPG",
        method: Deflated,
        level: Some(9),
    },
    Extension {
        name: "JPEG",
        method: Deflated,
        level: Some(9),
    },
    Extension {
        name: "PNG",
        method: Stored,
        level: None,
    },
    Extension {
        name: "WEBP",
        method: Stored,
        level: None,
    },
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

    for path in &args.dirs {
        let mut images = vec![];
        for dir in read_dir(path)? {
            let path = dir?.path();
            if !path.is_dir()
                && path
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(str::to_uppercase)
                    .filter(|s| EXTENSIONS.iter().any(|e| e.name == s))
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

        let file = File::create(format!("{}.cbz", path.display()))?;
        let mut zip = ZipWriter::new(file);

        for path in targets {
            let name = path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or("invalid file name")?;
            let ext = path
                .extension()
                .and_then(OsStr::to_str)
                .and_then(|s| EXTENSIONS.iter().find(|e| e.name == s))
                .ok_or("invalid extension")?;

            let options = SimpleFileOptions::default()
                .compression_method(ext.method)
                .compression_level(ext.level);

            zip.start_file(name, options)?;
            let mut file = File::open(&path)?;
            copy(&mut file, &mut zip)?;
            println!("{}", &name);
        }

        zip.flush()?;
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
