use std::{path::{PathBuf, Path}, collections::HashMap, io::Write, env::current_dir, fs::create_dir_all};

use clap::ArgMatches;

use crate::{hash::{hash2path, hash_files, HashType}, files::{LogLevel, writefile, acknowledge}};



pub fn run(
    args: &ArgMatches,
    paths: &[PathBuf],
    source_hashes: &HashMap<PathBuf, (PathBuf, String)>, // partial hashes for pruning unique hashes quickly
    hash_type: &HashType,
    verbose: bool
) -> std::io::Result<()> {

    let log_level = LogLevel::from(*args.get_one::<bool>("log").unwrap());

    print!("Pruning unique hashes...");
    std::io::stdout().flush()?;
    let pruned_hashes: HashMap<String, Vec<PathBuf>> = hash2path(source_hashes).into_iter()
        .filter(|(_hash, paths)| paths.len() > 1) // remove unique files
        .collect();
    let pruned_paths: Vec<_> = pruned_hashes.values()
        .flatten()
        .cloned()
        .collect();
    println!(" Done (pruned {} unique files)", source_hashes.len() - pruned_paths.len());

    println!("\nHashing remaining files in full...");
    let duplicate_hashes = hash_files(
        &pruned_paths,
        " DUPL ",
        &hash_type,
        verbose,
        None,
        None,
    )?;
    println!("Done ({} files)\n", duplicate_hashes.len());

    // Duplicate files, somewhat odd structure: HASH\t\FILE1\tFILE2\t... (columns will vary depending on number of duplicates)
    let mut log_duplicates: Vec<String> = Vec::new();
    // For determining number of columns/headers
    let mut log_duplicates_max = 0_usize;
    let mut log_duplicates_ext = "<NONE>".to_owned();

    let duplicate_paths = hash2path(&duplicate_hashes);
    let mut dupe_hash_count = 0;
    let mut dupe_paths: Vec<PathBuf> = Vec::new();
    // hashes, paths that have already been filtered from unique instances
    for (hash, paths) in duplicate_paths.iter() {
        log_duplicates.push(format!("{hash}\t{}", paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\t")));
        if paths.len() > log_duplicates_max {
            log_duplicates_max = paths.len();
            // // At least a sinc
            log_duplicates_ext = paths.first().unwrap() // at least 2 paths should exist, presumbly same file ext...
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }
        if paths.len() > 1 { // required, but already filtered these above in pruned_hashes?
            dupe_hash_count += 1;
            dupe_paths.extend(paths.to_owned());
            println!("[{:5} | {} HASH: {}]", dupe_hash_count, hash_type.to_string(), hash);
            for path in paths.iter() {
                println!("    {}", path.display());
            }
        }
    }

    println!("\nSummary:");
    println!("  Files, total:           {}", paths.len());
    println!("  Duplicate files, total: {}", dupe_paths.len());
    println!("  Duplicate hashes:       {}", dupe_hash_count);

    if log_level == LogLevel::Normal {
        let log_dir = match args.get_one::<PathBuf>("log-dir") {
            Some(d) => d.to_owned(), // must exist
            None => {
                let dir = current_dir()?.join("chksum_logs");
                create_dir_all(&dir)?;
                dir
            }
        };

        let log_path = log_dir.join(Path::new("duplicates.csv"));

        if log_duplicates_max > 0 { // will only be > 0 if duplicates is set AND duplicate files have been found
            if log_duplicates_max > 50 {
                let msg = format!(
                    "(!) File type '{log_duplicates_ext}' results in {log_duplicates_max} columns (one per duplicated file). Write log anyway?"
                );
                if acknowledge(&msg)? == false {
                    println!("User aborted writing log.");
                    return Ok(())
                }
            }
            let headers = format!("{}HASH\t{}",
                hash_type.to_string(),
                (0..log_duplicates_max).into_iter()
                    .map(|i| format!("FILE{}\t", i+1))
                    .collect::<String>()
            );
            match writefile(&format!("{}\n", vec![headers, log_duplicates.join("\n")].join("\n")), &log_path) {
                Ok(true) => println!("Wrote {}", log_path.display()),
                Ok(false) => println!("Aborted writing CSV."),
                Err(err) => println!("(!) Failed to write {}: {err}", log_path.display()),
            }
        }
    }

    // Show distribution for duplicates
    return super::stats::run(args, &dupe_paths)
}