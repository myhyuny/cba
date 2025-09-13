use clap::Parser;
use lazy_regex::{lazy_regex, Lazy, Regex};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    cmp::Ordering,
    collections::HashSet,
    ffi::OsStr,
    fs::{read_dir, File},
    io::{copy, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};
use tempfile::tempfile;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

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

    for path in &args.dirs {
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

        struct Image {
            file: File,
            name: String,
            compressed: bool,
        }

        let results = images
            .into_par_iter()
            .enumerate()
            .map(|(i, image)| -> Result<Image, Error> {
                let ext = image
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(|s| {
                        let e = s.to_uppercase();
                        return match e.as_str() {
                            "JPEG" => "JPG".to_owned(),
                            _ => e,
                        };
                    })
                    .ok_or("invalid file extension")?;

                let name = format!("{}{}.{}", "0".repeat(names - digit(i)), i, ext);
                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .compression_level(Some(15));

                let mut tmp = tempfile()?;

                let mut zip = ZipWriter::new(BufWriter::new(&tmp));
                zip.start_file(&name, options)?;

                let mut file = File::open(image)?;

                let mut reader = BufReader::new(&file);

                copy(&mut reader, &mut zip)?;
                zip.finish()?;

                let original = file.metadata()?.len();
                let compressed = tmp.metadata()?.len() - 22; // zip header size

                println!("{}, {} -> {}", &name, original, compressed);

                if original > compressed {
                    tmp.seek(SeekFrom::Start(0))?;

                    return Ok(Image {
                        file: tmp,
                        name,
                        compressed: false,
                    });
                } else {
                    file.seek(SeekFrom::Start(0))?;

                    return Ok(Image {
                        file,
                        name,
                        compressed: true,
                    });
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let file = File::create(format!("{}.cbz", path.display()))?;
        let buf = BufWriter::new(file);
        let mut zip = ZipWriter::new(buf);

        for image in results {
            if image.compressed {
                let options =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
                zip.start_file(image.name, options)?;
                let mut reader = BufReader::new(image.file);
                copy(&mut reader, &mut zip)?;
            } else {
                let mut archived = ZipArchive::new(image.file)?;
                let file = archived.by_index_raw(0)?;
                zip.raw_copy_file(file)?;
            }
        }

        zip.flush()?;
    }

    return Ok(());
}

fn digit(i: usize) -> usize {
    return (i as f64).log10().floor() as usize + 1;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
