use clap::{Arg, Command, ArgAction};
use time::OffsetDateTime;
use std::collections::{HashSet, HashMap};
use std::env::current_dir;
use std::fs::create_dir_all;
use std::io::Write;
use std::path::{PathBuf, Path};

use crate::datetime::{datetime_modified, now_to_string};
use crate::files::{paths, file_count, filename_to_string, writefile, LogLevel, file_stats};
use crate::hash::{HashType, hash2path, hash_files};

mod hash;
mod datetime;
mod files;
mod tasks;
mod errors;

const VERSION: &'static str = "0.4.8";

fn main() -> std::io::Result<()> {
    let args = Command::new("chksum")
        .version(VERSION)
        .author("Jens Larsson <jenslar@fastmail.com>")
        .term_width(80)
        .about("Calculate SHA256 or BLAKE3 checksum for all files in source directory recursively.
Optionally compare and match with all files in target directory recursively. Other uses are finding duplicate files,
or list an overview of total size of file types (file extension) in source directory.

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
        .arg(Arg::new("exclude-dir")
            .help("Exclude any sub-directory with this name.")
            .long("exclude-dir")
            .alias("ed")
            .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("include-ext")
            .help("File extensions to consider. Ignores all other files.")
            .long("include-ext")
            .alias("ie")
            .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("exclude-ext")
            .help("File extensions to exclude.")
            .long("exclude-ext")
            .alias("ee")
            .num_args(1..)
            .value_parser(clap::value_parser!(String)))
        .arg(Arg::new("log")
            // .help("Log hashes and paths as tab-separated text files. Can be used as input for 'source-hashes' and 'target-hashes'.")
            .help("Log hashes and paths as tab-separated text files.")
            .short('l')
            .long("log")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("log-dir")
            .help("Custom log directory.")
            .long("log-dir")
            .alias("ld")
            .value_parser(clap::value_parser!(PathBuf)))
        // .arg(Arg::new("log-level")
        //     // .help("Log hashes and paths as tab-separated text files. Can be used as input for 'source-hashes' and 'target-hashes'.")
        //     .help("Log hashes and paths as tab-separated text files.")
        //     .alias("ll")
        //     .long("log-level")
        //     .requires("log")
        //     .default_value("normal")
        //     .value_parser(PossibleValuesParser::new([
        //         "verbose",
        //         "normal"
        //     ])))
        .arg(Arg::new("duplicates")
            .help("Find duplicate files.")
            .long("duplicates")
            .conflicts_with("target-dir")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("case-sensitive")
            .help("Case sensitive file extensions. Count e.g. 'mp4' and 'MP4' separately. Only valid if 'count' is passed.")
            .long("case")
            .requires("count")
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
            .help("Use the faster Blake3 hashing algorithm instead of the default SHA256.")
            .long("blake3")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("stats")
            .help("Returns an overview of source-dir.")
            .long("stats")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("threshold")
            .help("Threshold in percent. Filetypes below threshold volume will not be shown for 'stats'.")
            .long("threshold")
            .default_value("1.0")
            .value_parser(clap::value_parser!(f64)))
        .arg(Arg::new("stats-sort-count")
            .help("Sort file stats on file extension count.")
            .long("sort-count")
            .alias("sc")
            .requires("stats")
            .conflicts_with_all(["stats-sort-size", "stats-sort-alpha"])
            .action(ArgAction::SetTrue))
        .arg(Arg::new("stats-sort-size")
            .help("Sort file stats on total file size.")
            .long("sort-size")
            .alias("sz")
            .requires("stats")
            .conflicts_with_all(["stats-sort-count", "stats-sort-alpha"])
            .action(ArgAction::SetTrue))
        .arg(Arg::new("stats-sort-alpha")
            .help("Sort file stats on file extension alphabetically.")
            .long("sort-alpha")
            .alias("sa")
            .requires("stats")
            .conflicts_with_all(["stats-sort-count", "stats-sort-size"])
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
    let exclude_dir: Vec<String> = args.get_many("exclude-dir").unwrap_or_default().cloned().collect();
    let include_ext: Vec<String> = args.get_many("include-ext").unwrap_or_default().cloned().collect();
    let exclude_ext: Vec<String> = args.get_many("exclude-ext").unwrap_or_default().cloned().collect();
    let duplicates = *args.get_one::<bool>("duplicates").unwrap();
    let fileext_case_sensitive = *args.get_one::<bool>("case-sensitive").unwrap();
    let filestats = *args.get_one::<bool>("stats").unwrap();
    let verbose = *args.get_one::<bool>("verbose").unwrap();

    let log_level = LogLevel::from(*args.get_one::<bool>("log").unwrap());
    let include_hidden = *args.get_one::<bool>("include-hidden").unwrap();
    let follow_symlinks = *args.get_one::<bool>("follow-symlinks").unwrap();
    let partial_hash_size = *args.get_one::<usize>("partial-hash-size").unwrap(); // clap default 1000
    // let match_filename = *args.get_one::<bool>("match-filename").unwrap();

    let hash_type = match args.get_one::<bool>("blake3").unwrap() {
        true => HashType::Blake3,
        false => HashType::Sha256
    };

    // All entries in source dir
    let mut log_source = vec![format!(
        "FILENAME\tSOURCEPATH\t{}\tDATETIME",
        hash_type.to_string()
    )];
    // All entries in target dir
    let mut log_target = vec![format!(
        "FILENAME\tTARGETPATH\t{}\tDATETIME",
        hash_type.to_string()
    )];
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

    // ALL TASKS

    // Compile paths first to enable parallel processing when hashing
    // Note that this makes it impossible to write incremental logs or enumerate output in order while hashing...
    print!("[ {} | {} ] Compiling paths...", if duplicates {"DUPCHK"} else {"SOURCE"}, source_dir.display());
    std::io::stdout().flush()?;
    let source_paths = paths(
        &source_dir,
        include_hidden,
        follow_symlinks,
        &exclude_dir,
        &include_ext,
        &exclude_ext,
    );
    source_count = source_paths.len();
    println!(" Done ({} files)", source_count);

    // RUN FILE STATS
    // No hashes needed, returns early
    if filestats {
        return tasks::stats::run(&args, &source_paths);
    }

    // REMAINING TASKS
    // Require some form of initial hashing of all files

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
    )?;

    // Write all source hashes as CSV to disk
    if log_level == LogLevel::Normal {
        for (path, hash) in source_hashes.values() {
            log_source.push(format!("{}\t{}\t{}\t{}",
                filename_to_string(path).unwrap_or("FILENAME ERROR".to_owned()),
                path.display(),
                hash,
                now_to_string()
            ))
        }
    }

    println!("{} ({} files{})\n",
        if duplicates {"Duplicate quick check done"} else {"Done"},
        source_hashes.len(),
        if duplicates {format!(" @ {} bytes each", dupl_quickcheck_size.unwrap())} else {"".to_string()}
    );

    // CHECK IF DUPLICATES SET: PRE-HASH, PRUNE, HASH FILES IN SOURCE DIR
    // Duplicate files, somewhat odd structure: HASH\t\FILE1\tFILE2\t... (columns will vary depending on number of duplicates)
    // let mut log_duplicates: Vec<String> = Vec::new();
    // // For determining number of columns/headers
    // let mut log_duplicates_max = 0_usize;

    if duplicates {

        // (log_duplicates, log_duplicates_max) = tasks::duplicates::run(&args, &source_paths, &source_hashes, &hash_type, verbose)?;
        return tasks::duplicates::run(&args, &source_paths, &source_hashes, &hash_type, verbose);

    // CHECK IF TARGET DIR SET, HASH FILES FOR COMPARING WITH SOURCE DIR
    }
    
    if let Some(tdir) = target_dir {

        print!("[ TARGET | {} ] Compiling paths...", tdir.display());
        let target_paths = paths(
            tdir,
            include_hidden,
            follow_symlinks,
            &exclude_dir,
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
        )?;
        println!("Done ({} files)\n", target_hashes.len());

        // find files not in target dir
        for (source_path, (_full_source_path, source_hash)) in source_hashes.iter() {
            match target_hashes.get(source_path) {
                Some((full_target_path, target_hash)) => {

                    // log all target files
                    if log_level == LogLevel::Normal {
                        log_target.push(format!("{}\t{}\t{}\t{}",
                            filename_to_string(full_target_path).unwrap_or("FILENAME ERROR".to_owned()),
                            full_target_path.display(),
                            target_hash,
                            now_to_string()
                        ))
                    }

                    // matching files
                    if source_hash == target_hash {
                        // log matching files
                        if log_level == LogLevel::Normal {
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
                    if log_level == LogLevel::Normal {
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
                if log_level == LogLevel::Normal {
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
                    .unwrap_or("Failed to retrieve modification date".to_owned());
                    // .expect("Failed to retrieve modification date");
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

                if log_level == LogLevel::Normal {
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
        println!("{:4} files missing in source", ignored.len());
    }
    
    if log_level == LogLevel::Normal {
        let log_matched_path: PathBuf;

        let log_dir = match args.get_one::<PathBuf>("log-dir") {
            Some(d) => d.to_owned(), // must exist
            None => {
                let dir = current_dir()?.join("chksum_logs");
                create_dir_all(&dir)?;
                dir
            }
        };

        if let Some(_t) = target_dir {
            log_matched_path = log_dir.join(Path::new("matched.csv"));
            let log_missing_path = log_dir.join(Path::new("missing_in_target.csv"));
            let log_changed_path = log_dir.join(Path::new("changed_in_target.csv"));
            let log_ignored_path = log_dir.join(Path::new("missing_in_source.csv")); // ignored
            let log_source_path = log_dir.join("checksums_source.csv");
            let log_target_path = log_dir.join("checksums_target.csv");

            match writefile(&log_source.join("\n"), &log_source_path) {
                Ok(true) => println!("Wrote {}", log_source_path.display()),
                Ok(false) => println!("Write aborted by user"),
                Err(err) => eprintln!("(!) Failed to write {}: {err}", log_source_path.display()),
            }

            match writefile(&log_target.join("\n"), &log_target_path) {
                Ok(true) => println!("Wrote {}", log_target_path.display()),
                Ok(false) => println!("Write aborted by user"),
                Err(err) => eprintln!("(!) Failed to write {}: {err}", log_target_path.display()),
            }

            if missing.len() > 0 {
                match writefile(&format!("{}\n", log_missing.join("\n")), &log_missing_path) {
                    Ok(true) => println!("Wrote {}", log_missing_path.display()),
                    Ok(false) => println!("Aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_missing_path.display())
                }
            } else {
                println!("No missing files. Skipping CSV.")
            }

            if changed.len() > 0 {
                match writefile(&format!("{}\n", log_changed.join("\n")), &log_changed_path) {
                    Ok(true) => println!("Wrote {}", log_changed_path.display()),
                    Ok(false) => println!("Aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_changed_path.display())
                }
            } else {
                println!("No changed files. Skipping CSV.")
            }

            if ignored.len() > 0 {
                match writefile(&format!("{}\n", log_ignored.join("\n")), &log_ignored_path) {
                    Ok(true) => println!("Wrote {}", log_ignored_path.display()),
                    Ok(false) => println!("Aborted writing CSV."),
                    Err(err) => eprintln!("(!) Failed to write {}: {err}", log_ignored_path.display())
                }
            } else {
                println!("No ignored files. Skipping CSV.")
            }

        // } else if duplicates {
        //     log_matched_path = log_dir.join(Path::new("duplicates.csv"));
        } else {
            log_matched_path = log_dir.join(Path::new("checksums.csv"));
        }

        // if log_duplicates_max > 0 { // will only be > 0 if duplicates is set AND duplicate files have been found
        //     let headers = format!("{}HASH\t{}",
        //         hash_type.to_string(),
        //         (0..log_duplicates_max).into_iter()
        //             .map(|i| format!("FILE{}\t", i+1))
        //             .collect::<String>()
        //     );
        //     match writefile(&format!("{}\n", vec![headers, log_duplicates.join("\n")].join("\n")), &log_matched_path) {
        //         Ok(true) => println!("Wrote {}", log_matched_path.display()),
        //         Ok(false) => println!("Aborted writing CSV."),
        //         Err(err) => println!("(!) Failed to write {}: {err}", log_matched_path.display()),
        //     }
        // } else {
        match writefile(&format!("{}\n", log_matched.join("\n")), &log_matched_path) {
            Ok(true) => println!("Wrote {}", log_matched_path.display()),
            Ok(false) => println!("Aborted writing CSV."),
            Err(err) => println!("(!) Failed to write {}: {err}", log_matched_path.display()),
        }
        // }
    }

    Ok(())
}
