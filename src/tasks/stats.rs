use std::{path::PathBuf, collections::HashMap};

use clap::ArgMatches;

use crate::files::file_stats;

pub fn run(args: &ArgMatches, paths: &[PathBuf]) -> std::io::Result<()> {
    let fileext_case_sensitive = *args.get_one::<bool>("case-sensitive").unwrap();
    let filestats_sort_count = *args.get_one::<bool>("stats-sort-count").unwrap();
    let filestats_sort_size = *args.get_one::<bool>("stats-sort-size").unwrap();

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
        
    match (filestats_sort_count, filestats_sort_size) {
        (true, false) => stats_vec.sort_by_cached_key(|(_, _, c)| *c),
        (false, true) => stats_vec.sort_by_cached_key(|(_, s, _)| *s),
        _ => stats_vec.sort_by_cached_key(|(_, _, c)| *c),
    }

    for (ext, size, count) in stats_vec.iter() {
        println!("{ext:>22} {count:<13} {size:>13} bytes [{:>3.3}%]", (*size * 100) as f64 / total_size as f64);
    }
    println!("---");
    println!("{total_size:>50} bytes");

    Ok(())
}