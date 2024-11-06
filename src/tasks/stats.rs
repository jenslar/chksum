use std::{collections::HashMap, path::PathBuf};

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
            .map(|(e, s, _created, _modified)| (e.unwrap_or("< no ext >".to_owned()), s))?;

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

    // █ ▓ ▒ ░
    // list extensions + size
    for (ext, size, count) in stats_vec.iter() {
        // size of total in percent, if filetypes specified, show all
        if args.contains_id("include-ext") {
            threshold = 0.0;
        }
        let relative_size = (*size * 100) as f64 / total_size as f64;

        // 100 blocks = total size, perhaps lower for term width
        let blocks_full = relative_size.floor();
        let blocks_rem = relative_size - blocks_full as f64;
        // only show file types above a certain total size, defaults to 1%
        if relative_size > threshold {
            print!("{ext:>21} {count:<10} {:>10} [{:>6}%] ",
                Units::from(**size).to_string(),
                format!("{relative_size:3.3}") // ugly but works for alignment...
            );
            // if only a single extension
            let block_partial = if stats_vec.len() == 1 {
                ""
            } else {
                match blocks_rem {
                    0.0..0.25 => "░",
                    0.25..0.50 => "▒",
                    0.50..0.75 => "▓",
                    _ => "█"
                }
            };
            let block_string = format!("{}{block_partial}", "█".repeat(blocks_full as usize));
            println!("{block_string:<100}");
        } else {
            other_count += *count;
            other_size += *size;
        }
    }
    if other_size > 0 {
        let other_relative_size = (other_size * 100) as f64 / total_size as f64;

        print!("{:>21} {other_count:<10} {:>10} [{:>6}%] ",
            "< other >",
            Units::from(other_size).to_string(),
            format!("{other_relative_size:3.3}") // ugly but works for alignment...
        );

        let blocks_full = other_relative_size.floor();
        let blocks_rem = other_relative_size - blocks_full as f64;
        // checking single ext not needed here
        let block_partial = match blocks_rem {
            0.0..0.33 => "░",
            0.33..0.66 => "▒",
            0.66..1.0 => "▓",
            _ => ""
        };
        let block_string = format!("{}{block_partial}", "█".repeat(blocks_full as usize));
        println!("{block_string:<100}");
    }
    println!("---");
    println!("Total                 {:<13} {:>7}", paths.len(), Units::from(total_size).to_string());
    if args.contains_id("include-ext") {
        println!("\n'include-ext' set: showing total relative size for all included file types.")
    } else {
        println!("\nFile types below {:.1}% of total size grouped as '< other >'.\nUse '--threshold <PERCENTAGE>' to change this behaviour.",
            threshold
        );
    }

    Ok(())
}

pub enum Units {
    Bytes(u64),
    Kilo(f64),
    Mega(f64),
    Giga(f64),
    Tera(f64),
}

impl From<u64> for Units {
    fn from(value: u64) -> Self {
        match value as f64 {
            _z @ ..1e3 => Self::Bytes(value),
            z @ 1e3..1e6 => Self::Kilo(z / 1e3),
            z @ 1e6..1e9 => Self::Mega(z / 1e6),
            z @ 1e9..1e12 => Self::Giga(z / 1e9),
            z @ 1e12..1e15 => Self::Tera(z / 1e12),
            z => Self::Tera(z / 1e12),
        }
    }
}

impl std::fmt::Display for Units {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Units::Bytes(n) => write!(f, "{n} bytes"),
            Units::Kilo(fl) => write!(f, "{fl:.2}KB", ),
            Units::Mega(fl) => write!(f, "{fl:.2}MB", ),
            Units::Giga(fl) => write!(f, "{fl:.2}GB", ),
            Units::Tera(fl) => write!(f, "{fl:.2}TB", ),
        }
    }
}