use std::fs::File;
use std::path::Path;

use crc32fast::Hasher;
use memmap::Mmap;
use regex::Regex;
use walkdir::WalkDir;

fn hash<P: AsRef<Path>>(path: P) -> std::io::Result<u32> {
    let f = File::open(path.as_ref())?;
    let buf = unsafe { Mmap::map(&f)? };

    let mut hasher = Hasher::new();
    hasher.update(&buf);
    let checksum = hasher.finalize();

    Ok(checksum)
}

fn check_file<T: AsRef<str>>(path: T, should_print_name: bool) -> std::io::Result<()> {
    let path = path.as_ref();

    let sum = hash(path)?;
    print!("{:08X}", sum);

    if should_print_name {
        print!("\t{}", path);
    }

    const PAT: &str = "[[:^xdigit:]]([[:xdigit:]]{8})[[:^xdigit:]]";
    let re = Regex::new(PAT).unwrap();

    if let Some(caps) = re.captures(path) {
        let filename_sum = u32::from_str_radix(&caps[1], 16).unwrap();
        if sum == filename_sum {
            print!(" \tOK");
        } else {
            print!(" \tBAD {:08X} != {:08X}", sum, filename_sum);
        }
    }

    println!();
    Ok(())
}

fn check_item(path: &str, multiple_args: bool) -> std::io::Result<()> {
    let metadata = std::fs::metadata(path)?; // folllows symlinks
    if metadata.is_file() {
        check_file(path, multiple_args)?;
    } else {
        assert!(metadata.is_dir());
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let path = entry.path().to_str().expect("invalid unicode in path");
                    check_file(path, true)?;
                }
            }
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    for arg in &args {
        check_item(arg, args.len() > 1)?;
    }

    Ok(())
}
