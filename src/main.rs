use clap::{Arg, Command, ArgAction};
use std::collections::{HashSet, HashMap};
use std::io::Write;
use std::path::{PathBuf, Path};

use crate::datetime::{datetime_modified, now_to_string};
use crate::files::{paths, file_count, filename_to_string, writefile};
use crate::hash::{HashType, hash2path, hash_files};

mod hash;
mod datetime;
mod files;

fn main() -> std::io::Result<()> {
    let args = Command::new("chksum")
        .version("0.3.0")
        .author("Jens Larsson <jenslar@fastmail.com>")
        .term_width(80)
        .about("Calculate SHA256 or BLAKE3 checksum for all files in SOURCE directory recursively,
and optionally compare and match with all files in TARGET directory recursively. Matches checksum and relative path in SOURCE and TARGET
by default.

NOTE: Sha256 checksums do not match Blake3 checksums. Blake3 is the faster of the two. Install the 'b3sum'
utility if there is a need to verify Blake3 checksums for individual files (https://github.com/BLAKE3-team/BLAKE3).")
        .arg(Arg::new("source-dir")
            .help("Calculate checksums for all files in this path recursively.")
            .short('s')
            .long("source-dir")
            .value_parser(clap::value_parser!(PathBuf))
            .required(true))
        .arg(Arg::new("target-dir")
            .help("If passed, file checksums will be compared to those for source-dir.")
            .short('t')
            .long("target-dir")
            .value_parser(clap::value_parser!(PathBuf)))
        // .arg(Arg::new("source-hashes")
        //     .help("Hashes for source files. Any file hash listed will not be re-calculated as a means of continuing interrupted runs.")
        //     .alias("sh")
        //     .long("source-hashes")
        //     .value_parser(clap::value_parser!(PathBuf)))
        // .arg(Arg::new("target-hashes")
        //     .help("Hashes for target files. Any file hash listed will not be re-calculated as a means of continuing interrupted runs.")
        //     .alias("th")
        //     .long("target-hashes")
        //     .value_parser(clap::value_parser!(PathBuf)))
        .arg(Arg::new("ignore-dir")
            .help("Ignore any sub-directory with this name.")
            .short('i')
            .long("ignore-dir")
            // TODO should allow multiple ignore dirs
            // .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("include-ext")
            .help("File extensions to consider. Ignores all other files.")
            .long("include")
            .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("exclude-ext")
            .help("File extensions to exclude.")
            .long("exclude")
            .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("log")
            // .help("Log hashes and paths as tab-separated text files. Can be used as input for 'source-hashes' and 'target-hashes'.")
            .help("Log hashes and paths as tab-separated text files.")
            .short('l')
            .long("log")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("duplicates")
            .help("Find duplicate files.")
            .long("duplicates")
            .conflicts_with("target-dir")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("count")
            .help("Print count for each file extension.")
            .long("count")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("verbose")
            .help("Print each encountered file.")
            .short('v')
            .long("verbose")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("include-hidden")
            .help("Include hidden files (filename starts with '.'). Currently only works for unix/linux.")
            .long("hidden")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("follow-symlinks")
            .help("Follow symlinks. Symlinks will otherwise be ignored.")
            .long("symlinks")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("partial-hash-size")
            .help("Partial hash size for duplicate quick check.")
            .long("parsize")
            .default_value("1000")
            .requires("duplicates")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new("blake3")
            .help("Use the much faster Blake3 hash function instead of the default SHA256.")
            .long("blake3")
            .action(ArgAction::SetTrue))
        // TODO reinstate possibility to check is FileA exists both in <DIRA> and <DIRB> regardless of path, i.e. report match if so
        // .arg(Arg::new("match-filename")
        //     .help("Compare filename instead of full path.")
        //     .long("filename")
        //     .requires("target-dir")
        //     .action(ArgAction::SetTrue))
        .get_matches();

    let source_dir = args.get_one::<PathBuf>("source-dir").unwrap(); // required arg
    let target_dir = args.get_one::<PathBuf>("target-dir");
    let ignore_dir = args.get_one::<String>("ignore-dir").map(|s| s.as_str());
    let include_ext: Vec<String> = args.get_many("include-ext").unwrap_or_default().cloned().collect();
    let exclude_ext: Vec<String> = args.get_many("exclude-ext").unwrap_or_default().cloned().collect();
    let duplicates = *args.get_one::<bool>("duplicates").unwrap();
    let fileext_count = *args.get_one::<bool>("count").unwrap();
    let verbose = *args.get_one::<bool>("verbose").unwrap();
    let log = *args.get_one::<bool>("log").unwrap();
    let include_hidden = *args.get_one::<bool>("include-hidden").unwrap();
    let follow_symlinks = *args.get_one::<bool>("follow-symlinks").unwrap();
    let partial_hash_size = *args.get_one::<usize>("partial-hash-size").unwrap(); // clap default 1000
    // let match_filename = *args.get_one::<bool>("match-filename").unwrap();

    let hash_type = match args.get_one::<bool>("blake3").unwrap() {
        true => HashType::Blake3,
        false => HashType::Sha256
    };

    let mut log_matched = vec![format!(
        "FILENAME\tTARGETPATH\t{}\tDATETIME",
        hash_type.to_string()
    )];
    let mut log_missing = vec![format!(
        "FILENAME\tSOURCEPATH\t{}\tDATETIME",
        hash_type.to_string()
    )];
    let mut log_ignored = vec![format!(
        "FILENAME\tTARGETPATH\t{}\tDATETIME",
        hash_type.to_string()
    )];
    let mut log_changed = vec![format!(
        "FILENAME\tSOURCEPATH\tSOURCE{0}\tSOURCEMODIFIED\tTARGETPATH\tTARGET{0}\tTARGETMODIFIED",
        hash_type.to_string()
    )];
    // Duplicate files, somewhat odd structure: HASH\t\FILE1\tFILE2\t... (columns will vary depending on number of duplicates)
    let mut log_duplicates: Vec<String> = Vec::new();
    // For determining number of columns/headers
    let mut log_duplicates_max = 0_usize;

    // only in target, assume new/updated
    let mut ignored: HashSet<PathBuf> = HashSet::new();
    // only in source, assume missing/not synced
    let mut missing: HashSet<PathBuf> = HashSet::new();
    // path in both source and target, but hash not matching, assume changed in target
    let mut changed: HashSet<PathBuf> = HashSet::new();
    // matching paths and hashes
    let mut matched: HashSet<PathBuf> = HashSet::new();

    let mut source_count = 0;
    let mut target_count = 0;

    print!("[ {} | {} ] Compiling paths...", if duplicates {"DUPCHK"} else {"SOURCE"}, source_dir.display());
    std::io::stdout().flush()?;
    let source_paths = paths(
        &source_dir,
        include_hidden,
        follow_symlinks,
        ignore_dir.as_deref(),
        &include_ext,
        &exclude_ext,
    );
    source_count = source_paths.len();
    println!(" Done ({} files)", source_count);

    if fileext_count && !duplicates {
        let extsorted = file_count(&source_paths, None);

        for (ext, count) in extsorted.iter() {
            println!("{ext:>22} {count:<}")
        }

        println!("{}", "-".repeat(30));
        println!("{:>22} {}", "Files, total", extsorted.iter().map(|(_, n)| n).sum::<usize>());
        println!("{:>22} {}", "File types", extsorted.len());

        return Ok(())
    }

    // If duplicates check: read only part of file, then prune unique hashes
    // to lessen the number of file to fully hash. Arbitrary 1000 bytes
    let dupl_quickcheck_size = match duplicates {
        true => Some(partial_hash_size),
        false => None,
    };

    println!("[ {} | {} ] Compiling {}hashes{}...",
        if duplicates {"DUPCHK"} else {"SOURCE"},
        source_dir.display(),
        if duplicates {"partial "} else {""},
        if duplicates {format!(" ({} bytes)", dupl_quickcheck_size.unwrap())} else {"".to_string()}
    );

    let source_hashes = hash_files(
        &source_paths,
        if duplicates {"DUPCHK"} else {"SOURCE"},
        &hash_type,
        verbose,
        dupl_quickcheck_size,
        Some(source_dir)
    );

    println!("{} ({} files{})\n",
        if duplicates {"Duplicate quick check done"} else {"Done"},
        source_hashes.len(),
        if duplicates {format!(" @ {} bytes each", dupl_quickcheck_size.unwrap())} else {"".to_string()}
    );

    if duplicates {

        print!("Pruning unique hashes...");
        std::io::stdout().flush()?;
        let pruned_hashes: HashMap<String, Vec<PathBuf>> = hash2path(&source_hashes).into_iter()
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
        );
        println!("Done ({} files)\n", duplicate_hashes.len());

        let duplicate_paths = hash2path(&duplicate_hashes);
        let mut dupe_hash_count = 0;
        let mut dupe_file_count = 0;
        for (hash, paths) in duplicate_paths.iter() {
            log_duplicates.push(format!("{hash}\t{}", paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\t")));
            if paths.len() > log_duplicates_max {
                log_duplicates_max = paths.len();
            }
            if paths.len() > 1 { // already filtered these above in pruned_hashes?
                dupe_hash_count += 1;
                dupe_file_count += paths.len();
                println!("[{:5} | {} HASH: {}]", dupe_hash_count, hash_type.to_string(), hash);
                for path in paths.iter() {
                    println!("    {}", path.display());
                }
            }
        }

        println!("\nSummary:");
        println!("  Files, total:           {}", source_count);
        println!("  Duplicate files, total: {}", dupe_file_count);
        println!("  Duplicate hashes:       {}", dupe_hash_count);
        if fileext_count {
            println!("  Distribution:");
            let ext_unique = file_count(&duplicate_paths.values().cloned().flatten().collect::<Vec<_>>(), Some(2));
            for (ext, count) in ext_unique.iter() {
                println!("{ext:>22} {count:<}")
            }
        }

    } else if let Some(tdir) = target_dir {

        print!("[ TARGET | {} ] Compiling paths...", tdir.display());
        let target_paths = paths(
            tdir,
            include_hidden,
            follow_symlinks,
            ignore_dir.as_deref(),
            &include_ext,
            &exclude_ext
        );
        target_count = target_paths.len();
        println!(" Done ({} files)", target_count);

        println!("[ TARGET | {} ] Compiling hashes...", tdir.display());
        let target_hashes = hash_files(
            &target_paths,
            "TARGET",
            &hash_type,
            verbose,
            None,
            Some(tdir)
        );
        println!("Done ({} files)\n", target_hashes.len());

        // find files not in target dir
        for (source_path, (_full_source_path, source_hash)) in source_hashes.iter() {
            match target_hashes.get(source_path) {
                Some((full_target_path, target_hash)) => {
                    // matching files
                    if source_hash == target_hash {
                        if log {
                            // filename, path, hash, datetime
                            log_matched.push(format!("{}\t{}\t{}\t{}",
                                filename_to_string(full_target_path).unwrap_or("FILENAME ERROR".to_owned()),
                                full_target_path.display(),
                                source_hash.to_owned(),
                                now_to_string()
                            ));
                        }
                        matched.insert(source_path.to_owned());
                    // path matches, but not hash, assume changed files
                    } else {
                        // do logging below instead
                        changed.insert(source_path.to_owned());
                    }
                },
                None => {
                    if log {
                        // filename, path, hash, datetime
                        log_missing.push(format!("{}\t{}\t{}\t{}",
                            filename_to_string(source_path).unwrap_or("FILENAME ERROR".to_owned()),
                            source_path.display(),
                            source_hash.to_owned(),
                            now_to_string()
                        ));
                    }
                    missing.insert(source_path.to_owned());}
            }
        }

        for (target_path, (_full_target_path, target_hash)) in target_hashes.iter() {
            // file path doesn't exist in source, assume new file
            if source_hashes.get(target_path).is_none() {
                if log {
                    // filename, path, hash, datetime
                    log_ignored.push(format!("{}\t{}\t{}\t{}",
                        filename_to_string(target_path).unwrap_or("FILENAME ERROR".to_owned()),
                        target_path.display(),
                        target_hash.to_owned(),
                        now_to_string()
                    ));
                }
                ignored.insert(target_path.to_owned());
            }
        }

        println!("Result: {}/{} files match", matched.len(), source_count);

        print!("\n{} files missing in target", missing.len());
        if missing.len() == 0 {
            println!("");
        } else {
            println!(":");
            for (i, path) in missing.iter().enumerate() {
                let full_path = source_dir.join(path);
                println!("  [ MISSING {:5} ] {}", i+1, full_path.display())
            }
        }

        print!("\n{} files changed in target", changed.len());
        if changed.len() == 0 {
            println!("");
        } else {
            println!(":");
            for (i, path) in changed.iter().enumerate() {

                let full_path = tdir.join(path);

                let (source_path, source_hash) = source_hashes.get(path)
                    .expect("Failed to get source path");
                let source_modified = datetime_modified(&source_path)
                    .expect("Failed to retrieve modification date");
                let source_size = source_path.metadata()
                    .expect("Failed to determine source size")
                    .len();

                let (target_path, target_hash) = target_hashes.get(path)
                    .expect("Failed to get target path");
                let target_modified = datetime_modified(&target_path)
                    .expect("Failed to retrieve modification date");
                let target_size = target_path.metadata()
                    .expect("Failed to determine target size")
                    .len();

                if log {
                    log_changed.push(format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t",
                        filename_to_string(source_path).unwrap_or("FILENAME ERROR".to_owned()),
                        source_path.display(),
                        source_hash.to_owned(),
                        source_modified,
                        target_path.display(),
                        target_hash.to_owned(),
                        target_modified,
                    ));
                }

                println!("  [ CHANGED {:5} ] {}", i+1, full_path.display());
                println!("    SOURCE {source_modified} {source_size:>12} bytes {}",
                    source_path.display());
                println!("    TARGET {target_modified} {target_size:>12} bytes {}",
                    target_path.display());
            }
        }

        print!("\n{} files not in source", ignored.len());
        if ignored.len() == 0 {
            println!("");
        } else {
            println!(":");
            for (i, path) in ignored.iter().enumerate() {
                let full_path = tdir.join(path);
                println!("  [ IGNORED {:5} ] {}", i+1, full_path.display())
            }
        }

        println!("\nSummary");
        println!("{}/{} files match", matched.len(), source_count);
        println!("{:4} files missing in target", missing.len());
        println!("{:4} files changed in target", changed.len());
        println!("{:4} files not in source", ignored.len());
    }
    
    if log {
        let log_matched_path: PathBuf;

        if let Some(t) = target_dir {
            log_matched_path = Path::new(t).join(Path::new("checksums_matched.csv"));
            let log_missing_path = Path::new(t).join(Path::new("checksums_missing.csv"));
            let log_changed_path = Path::new(t).join(Path::new("checksums_changed.csv"));
            let log_ignored_path = Path::new(t).join(Path::new("checksums_ignored.csv"));

            if missing.len() > 0 {
                match writefile(&format!("{}\n", log_missing.join("\n")), &log_missing_path) {
                    Ok(true) => println!("Wrote {}", log_missing_path.display()),
                    Ok(false) => println!("User aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_missing_path.display())
                }
            } else {
                println!("No missing files. Skipping CSV.")
            }

            if changed.len() > 0 {
                match writefile(&format!("{}\n", log_changed.join("\n")), &log_changed_path) {
                    Ok(true) => println!("Wrote {}", log_changed_path.display()),
                    Ok(false) => println!("User aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_changed_path.display())
                }
            } else {
                println!("No changed files. Skipping CSV.")
            }

            if ignored.len() > 0 {
                match writefile(&format!("{}\n", log_ignored.join("\n")), &log_ignored_path) {
                    Ok(true) => println!("Wrote {}", log_ignored_path.display()),
                    Ok(false) => println!("User aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_ignored_path.display())
                }
            } else {
                println!("No ignored files. Skipping CSV.")
            }

        } else if duplicates {
            log_matched_path = Path::new(source_dir).join(Path::new("duplicates.csv"));
        } else {
            log_matched_path = Path::new(source_dir).join(Path::new("checksums.csv"));
        }

        if log_duplicates_max > 0 { // will only be > 0 if duplicates is set AND duplicate files have been found
            let headers = format!("{}HASH\t{}",
                hash_type.to_string(),
                (0..log_duplicates_max).into_iter()
                    .map(|i| format!("FILE{}\t", i+1))
                    .collect::<String>()
            );
            match writefile(&format!("{}\n", vec![headers, log_duplicates.join("\n")].join("\n")), &log_matched_path) {
                Ok(true) => println!("Wrote {}", log_matched_path.display()),
                Ok(false) => println!("User aborted writing CSV."),
                Err(err) => println!("(!) Failed to write {}: {err}", log_matched_path.display()),
            }
        } else {
            match writefile(&format!("{}\n", log_matched.join("\n")), &log_matched_path) {
                Ok(true) => println!("Wrote {}", log_matched_path.display()),
                Ok(false) => println!("User aborted writing CSV."),
                Err(err) => println!("(!) Failed to write {}: {err}", log_matched_path.display()),
            }
        }
    }

    Ok(())
}
