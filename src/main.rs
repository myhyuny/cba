use clap::Parser;
use lazy_regex::{lazy_regex, Lazy, Regex};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    cmp::Ordering,
    collections::HashSet,
    ffi::OsStr,
    fs::{read_dir, rename, File},
    io::{copy, BufReader, BufWriter, Write},
    path::PathBuf,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

static NUMBER_REGEX: Lazy<Regex> = lazy_regex!(r"(\d+)");

static EXTENSIONS: Lazy<HashSet<String>> = Lazy::new(|| {
    ["AVIF", "GIF", "JPG", "JPEG", "PNG", "WEBP"]
        .iter()
        .map(|&s| s.to_string())
        .collect()
});

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

    args.dirs
        .par_iter()
        .try_for_each(|path| -> Result<(), Error> {
            let mut images = Vec::new();
            for dir in read_dir(path)? {
                let path = dir?.path();
                if !path.is_dir()
                    && path
                        .extension()
                        .and_then(OsStr::to_str)
                        .map(str::to_uppercase)
                        .filter(|s| EXTENSIONS.contains(s))
                        .is_some()
                {
                    images.push(path);
                }
            }
            if images.is_empty() {
                return Ok(());
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

            let sources = if !targets.iter().any(|p| images.contains(p)) {
                images
            } else {
                let parent = path.display();
                let targets: Vec<PathBuf> = images
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
            let buf = BufWriter::new(file);
            let mut zip = ZipWriter::new(buf);

            for path in targets {
                let name = path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .ok_or("invalid file name")?;

                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .compression_level(Some(15));

                zip.start_file(name, options)?;
                let file = File::open(&path)?;
                let mut reader = BufReader::new(file);
                copy(&mut reader, &mut zip)?;
                println!("{}", &name);
            }

            zip.flush()?;

            return Ok(());
        })?;

    return Ok(());
}

fn rename_all(sources: &[PathBuf], targets: &[PathBuf]) -> Result<(), Error> {
    for i in 0..usize::min(sources.len(), targets.len()) {
        rename(&sources[i], &targets[i])?;
    }
    return Ok(());
}

fn digit(i: usize) -> usize {
    return (i as f64).log10().floor() as usize + 1;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
