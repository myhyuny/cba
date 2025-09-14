use clap::Parser;
use lazy_regex::{lazy_regex, Lazy, Regex};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    cmp::Ordering,
    collections::HashSet,
    ffi::OsStr,
    fs::{read_dir, File},
    io::{copy, BufReader, Write},
    path::PathBuf,
};
use tempfile::tempfile;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

static NUMBER_REGEX: Lazy<Regex> = lazy_regex!(r"(\d+)");

static EXTENSIONS: Lazy<HashSet<String>> = Lazy::new(|| {
    ["avif", "gif", "jpg", "jpeg", "png", "webp"]
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

    for path in &args.dirs {
        println!("{}", path.display());

        let mut images = Vec::new();
        for dir in read_dir(path)? {
            let path = dir?.path();
            if !path.is_dir()
                && path
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(str::to_lowercase)
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

        let results = images
            .into_par_iter()
            .enumerate()
            .map(|(i, image)| -> Result<(File, bool), Error> {
                let ext = image
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(|s| {
                        let e = s.to_uppercase();
                        return match e.as_str() {
                            "jpeg" => "jpg".to_owned(),
                            _ => e,
                        };
                    })
                    .ok_or("invalid file extension")?;

                let name = format!("{}{}.{}", "0".repeat(names - digit(i)), i, ext);

                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .compression_level(Some(100))
                    .with_zopfli_buffer(Some(1 << 20));

                let tmp = tempfile()?;

                let mut zip = ZipWriter::new(&tmp);
                zip.start_file(&name, options)?;

                let file = File::open(&image)?;
                let mut reader = BufReader::new(&file);

                copy(&mut reader, &mut zip)?;
                zip.finish()?;

                let (compress, origins) = {
                    let mut archive = ZipArchive::new(&tmp)?;
                    let file = archive.by_index_raw(0)?;
                    (file.compressed_size(), file.size())
                };

                println!(
                    "{} -> {}, {} -> {}",
                    image
                        .file_name()
                        .and_then(OsStr::to_str)
                        .ok_or("invalid filename")?,
                    name,
                    origins,
                    compress,
                );

                return Ok((tmp, compress < origins));
            })
            .collect::<Result<Vec<_>, _>>()?;

        let file = File::create(format!("{}.cbz", path.display()))?;
        let mut zip = ZipWriter::new(file);

        for (image, compressed) in results {
            let mut archive = ZipArchive::new(image)?;
            if compressed {
                let file = archive.by_index_raw(0)?;
                zip.raw_copy_file(file)?;
            } else {
                let mut file = archive.by_index(0)?;
                let options =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
                zip.start_file(file.name(), options)?;
                copy(&mut file, &mut zip)?;
            }

            zip.flush()?;
        }

        zip.finish()?;

        println!();
    }

    return Ok(());
}

fn digit(i: usize) -> usize {
    return (i as f64).log10().floor() as usize + 1;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
