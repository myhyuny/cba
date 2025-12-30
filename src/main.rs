use anyhow::{Context, Result};
use clap::Parser;
use lazy_regex::{Lazy, Regex, lazy_regex};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{File, read_dir},
    io::{BufReader, copy},
    path::PathBuf,
};
use tempfile::tempfile;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

static EXTENSIONS: [&str; 6] = ["avif", "gif", "jpeg", "jpg", "png", "webp"];

#[derive(Parser)]
struct Args {
    #[arg(required = true)]
    dirs: Vec<PathBuf>,
}

fn main() -> Result<()> {
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
                    .filter(|s| EXTENSIONS.binary_search(&s.as_ref()).is_ok())
                    .is_some()
            {
                images.push(path);
            }
        }
        if images.is_empty() {
            return Ok(());
        }

        images.sort_by(|a, b| {
            static NUMBER_REGEX: Lazy<Regex> = lazy_regex!(r"(\d+)");

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
        let max_padding = if names > 0 { names - 1 } else { 0 };
        let padding = "0".repeat(max_padding);

        let results = images
            .into_par_iter()
            .enumerate()
            .map(|(i, image)| -> Result<(File, bool)> {
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
                    .context("invalid file extension")?;

                let digit_count = digit(i);
                let padding_count = names.saturating_sub(digit_count);
                let name = format!("{}{}.{}", &padding[..padding_count], i, ext);

                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .compression_level(Some(264))
                    .with_zopfli_buffer(Some(1 << 20));

                let tmp = tempfile()?;

                let mut zip = ZipWriter::new(&tmp);
                zip.start_file(&name, options)?;

                let file = File::open(&image)?;
                let mut reader = BufReader::with_capacity(64 * 1024, &file);

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
                        .context("invalid filename")?,
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
        }

        zip.finish()?;

        println!();
    }

    return Ok(());
}

fn digit(i: usize) -> usize {
    return i.checked_ilog10().map_or(1, |d| d as usize + 1);
}
