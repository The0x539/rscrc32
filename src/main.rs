use std::fs::File;
use std::io::Result;

use memmap2::Mmap;
use rayon::prelude::*;

fn main() {
    let paths = std::env::args().skip(1).collect::<Vec<_>>();

    if paths.is_empty() {
        match hash_stdin() {
            Ok(crc) => println!("{crc:08X}"),
            Err(error) => eprintln!("ERR {error}"),
        }
    } else if paths.len() == 1 {
        let path = &paths[0];
        let result = hash_file(path);
        report_result(path, result, false);
    } else {
        let mut inputs = Vec::with_capacity(paths.len());
        let mut outputs = Vec::with_capacity(paths.len());

        for path in paths {
            let (tx, rx) = oneshot::channel();
            inputs.push((path, tx));
            outputs.push(rx);
        }

        // in the background, hash the files in no particular order...
        std::thread::spawn(move || {
            inputs.into_par_iter().for_each(|(path, tx)| {
                let result = hash_file(&path);
                let _ = tx.send((path, result));
            });
        });

        // ...but report the results in the order the files were listed
        for rx in outputs {
            let (path, result) = rx.recv().unwrap();
            report_result(&path, result, true);
        }
    }
}

fn hash_stdin() -> Result<u32> {
    let stdin = std::io::stdin().lock();
    let mmap = unsafe { Mmap::map(&stdin)? };

    #[cfg(unix)]
    mmap.advise(memmap2::Advice::Sequential)?;

    Ok(crc32fast::hash(&mmap))
}

fn hash_file(path: &str) -> Result<u32> {
    let f = File::open(path)?;
    let mmap = unsafe { Mmap::map(&f)? };

    #[cfg(unix)]
    mmap.advise(memmap2::Advice::Sequential)?;

    Ok(crc32fast::hash(&mmap))
}

fn report_result(path: &str, result: Result<u32>, print_path: bool) {
    if let Ok(crc) = result {
        print!("{crc:08X}");
    } else {
        print!("????????");
    }

    if print_path {
        print!("\t{path}")
    }

    match result {
        Ok(crc) => {
            if let Some(name_crc) = find_crc_in_name(path) {
                if crc == name_crc {
                    print!("\tOK");
                } else {
                    print!("\tBAD {crc:08X} != {name_crc:08X}");
                }
            }
        }
        Err(error) => print!("\tERR {error}"),
    }

    println!();
}

fn find_crc_in_name(filepath: &str) -> Option<u32> {
    filepath.as_bytes().windows(10).rev().find_map(parse_crc)
}

fn parse_crc(window: &[u8]) -> Option<u32> {
    if window.len() != 10 {
        return None;
    }

    if window[0].is_ascii_hexdigit() || window[9].is_ascii_hexdigit() {
        return None;
    }

    let s = std::str::from_utf8(&window[1..9]).ok()?;
    let crc = u32::from_str_radix(s, 16).ok()?;

    Some(crc)
}
