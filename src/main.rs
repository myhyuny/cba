use std::{cmp::Ordering, env, fs::{metadata, read_dir, rename}, io, path::PathBuf, process::{Command, exit, Stdio}};

use regex::Regex;

fn main() {
    let seven_zip = match [
        "7z",
        "/usr/local/bin/7z",
        "/opt/local/bin/7z",
        r"C:\Program Files\7-Zip\7z",
        r"C:\Program Files (x86)\7-Zip\7z",
    ].iter().find(|&&p| Command::new(p).stdout(Stdio::null()).spawn().is_ok()) {
        Some(&c) => c,
        None => exit_error("7-zip is not installed."),
    };

    let dirs = env::args().skip(1)
        .map(|arg| PathBuf::from(arg))
        .filter(|r| r.is_dir())
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        print_help();
        exit_error("Directory does not exist in the argument.");
    }

    let regex = Regex::new(r"(\d+)").unwrap();

    for dir in &dirs {
        let mut images = match read_dir(&dir) {
            Ok(dir) => dir
                .filter_map(|r| r.map(|e| e.path()).ok())
                .filter(|r| !r.is_dir())
                .filter(|r| match r.extension().map_or("", |s| s.to_str().unwrap()).to_uppercase().as_str() {
                    "JPG" | "JPEG" | "PNG" => true,
                    _ => false
                })
                .collect::<Vec<_>>(),
            Err(e) => exit_error(e),
        };
        if images.is_empty() { continue; }

        images.sort_by(|a, b| {
            let mut acm = regex.captures_iter(a.file_name().unwrap().to_str().unwrap());
            let mut bcm = regex.captures_iter(b.file_name().unwrap().to_str().unwrap());

            loop {
                if let (Some(an), Some(bn)) = (acm.next(), bcm.next()) {
                    let cmp = an[1].parse::<u32>().unwrap().cmp(&bn[1].parse::<u32>().unwrap());
                    if cmp != Ordering::Equal {
                        return cmp;
                    } else {
                        continue;
                    }
                }
                break;
            }

            return a.cmp(b);
        });

        let numbers = f64::ceil(f64::log10(images.len() as f64)) as usize;
        let targets = (0..images.len()).map(|i| {
            let num = i.to_string();
            let ext = match images[i].extension().unwrap().to_str().unwrap().to_uppercase().as_str() {
                "JPG" | "JPEG" => "JPG",
                "PNG" => "PNG",
                _ => "",
            };
            dir.join(format!("{}{}.{}", "0".repeat(numbers - num.len()), num, ext))
        }).collect::<Vec<_>>();

        let sources: Vec<PathBuf>;
        if targets.iter().find(|&f| images.contains(f)).is_none() {
            sources = images
        } else {
            sources = images.iter()
                .map(|b| PathBuf::from(format!(
                    "{}/_{}",
                    b.parent().unwrap().to_str().unwrap(),
                    b.file_name().unwrap().to_str().unwrap())))
                .collect();
            if let Err(e) = rename_all(&images, &sources) {
                exit_error(e);
            }
        }

        if let Err(e) = rename_all(&sources, &targets) {
            exit_error(e);
        }

        let mut args = vec![
            "a".to_string(),
            "-bd".to_string(),
            "-mx=9".to_string(),
        ];

        if targets.iter().map(|b| metadata(b).unwrap().len()).sum::<u64>() > 10_485_760 {
            args.push("-tzip".to_string());
            args.push(format!("{}.cbz", dir.display()));
        } else {
            args.push("-t7z".to_string());
            args.push("-m0=lzma2".to_string());
            args.push("-mfb=64".to_string());
            args.push("-md=32m".to_string());
            args.push("-ms=on".to_string());
            args.push(format!("{}.cb7", dir.display()));
        }

        for file in &targets {
            args.push(file.display().to_string());
        }

        if let Err(e) = Command::new(seven_zip).args(args).stdout(Stdio::null()).spawn() {
            exit_error(e);
        }
    }
}

fn rename_all(sources: &Vec<PathBuf>, targets: &Vec<PathBuf>) -> io::Result<()> {
    for i in 0..sources.len() {
        let source = &sources[i];
        let target = &targets[i];
        rename(source, target)?;
    }
    Ok(())
}

fn exit_error<T: ToString>(t: T) -> ! {
    println!("{}", t.to_string());
    println!();
    exit(1);
}

fn print_help() {
    println!("Usage: cba [dirs...]");
    println!();
}
