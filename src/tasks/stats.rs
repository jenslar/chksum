use std::{path::PathBuf, collections::HashMap};

use clap::ArgMatches;

use crate::files::file_stats;

pub fn run(args: &ArgMatches, paths: &[PathBuf]) -> std::io::Result<()> {
    let fileext_case_sensitive = *args.get_one::<bool>("case-sensitive").unwrap();
    let filestats_sort_count = *args.get_one::<bool>("stats-sort-count").unwrap();
    let filestats_sort_size = *args.get_one::<bool>("stats-sort-size").unwrap();
    let filestats_sort_alpha = *args.get_one::<bool>("stats-sort-alpha").unwrap();

    // any file type below threshold (percentage of total) will not be shown
    let mut threshold = *args.get_one::<f64>("threshold").unwrap();
    if threshold > 100. {
        let msg = format!("{threshold} is not a valid value for 'threshold'. Must be a between 0.0 - 100.0 (%)");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, msg))
    }

    let mut stats: HashMap<String, (u64, usize)> = HashMap::new();
    let mut total_size = 0;

    for path in paths.iter() {
        let (mut ext, size) = file_stats(path)
            .map(|(e, s, _created, _modified)| (e.unwrap_or("<NO FILE EXT>".to_owned()), s))?;

        total_size += size;

        if !fileext_case_sensitive {
            ext = ext.to_lowercase();
        }
        
        stats.entry(ext)
            .and_modify(|v| {
                v.0 += size;
                v.1 += 1;
            })
            .or_insert((size, 1));
    }

    let mut stats_vec = stats.iter()
        .map(|(ext, (size, count))| (ext, size, count))
        .collect::<Vec<_>>();
        
    match (filestats_sort_alpha, filestats_sort_size, filestats_sort_count) {
        (true, false, false) => stats_vec.sort_by_cached_key(|(a, _, _)| a.to_owned()),
        (false, true, false) => stats_vec.sort_by_cached_key(|(_, s, _)| *s),
        (false, false, true) => stats_vec.sort_by_cached_key(|(_, _, c)| *c),
        _ => stats_vec.sort_by_cached_key(|(_, _, c)| *c),
    }

    // other file type below threshold (count, byte size)
    let (mut other_count, mut other_size): (usize, u64) = (0, 0);
    for (ext, size, count) in stats_vec.iter() {
        // size of total in percent, if filetypes specified, show all
        if args.contains_id("include-ext") {
            threshold = 0.0;
        }
        let relative_size = (*size * 100) as f64 / total_size as f64;
        // only show file types above a certain total size, defaults to 1%
        if relative_size > threshold {
            println!("{ext:>22} {count:<13} {size:>13} bytes [{:>3.3}%]", relative_size);
        } else {
            other_count += *count;
            other_size += *size;
        }
    }
    if other_size > 0 {
        let other_relative_size = (other_size * 100) as f64 / total_size as f64;
        println!("{:>22} {other_count:<13} {other_size:>13} bytes [{:>3.3}%]", "<OTHER>", other_relative_size);
    }
    println!("---");
    println!("Total {:>20} files {total_size:>17} bytes", paths.len());
    if args.contains_id("include-ext") {
        println!("\n'include-ext' set: showing total relative size for all included file types.")
    } else {
        println!("\nFile types below {:.1}% of total size grouped as <OTHER>.\nUse '--threshold <PERCENTAGE>' to change this behaviour.",
            threshold
        );
    }

    Ok(())
}