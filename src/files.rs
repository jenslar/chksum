use std::{path::{Path, PathBuf}, ffi::{OsStr, OsString}, collections::HashMap, fs::File, io::Write};

use filetime::FileTime;
use time::{OffsetDateTime, ext::NumericalDuration};
use walkdir::{WalkDir, DirEntry};

/// Returns `true` if `DirEntry`:
/// - is/is not hidden, (`include_hidden` - currently only Unix/Linux)
/// - does not contain a component/dir name that equeals `ignore_dir`
/// - has a file extension specified in `include_ext`
/// - does not have a file extension specified in `exclude_ext`
pub fn include(
    direntry: &DirEntry,
    include_hidden: bool,
    include_ext: &[String],
    exclude_ext: &[String]
) -> bool {
    if direntry.path().is_dir() {
        return false
    }

    let is_hidden = direntry.file_name()
        .to_str()
        .map(|n| n.starts_with(".")) // unix/linux only, windows has attributes that need checking
        .unwrap_or(false);
    if !include_hidden && is_hidden {
        return false
    }

    if let Some(ext) = direntry.path().extension() {
        let ext_str = ext.to_string_lossy().to_ascii_lowercase();
        if !include_ext.is_empty() {
            return include_ext.contains(&ext_str)
        }

        if !exclude_ext.is_empty() {
            return !exclude_ext.contains(&ext_str)
        }
    } else {
        // Check to ignore files without extension whenever
        // include extentions are specified
        if !include_ext.is_empty() {
            return false
        }
    }
    
    true
}

fn contains_dir(direntry: &DirEntry, dirs: &[String]) -> bool {
    for dir in dirs.iter() {
        if direntry.path().components().any(|c| c == std::path::Component::Normal(OsStr::new(dir))) {
            return true
        }
    }
    false
}

pub fn paths(
    dir: &Path,
    include_hidden: bool,
    follow_links: bool,
    exclude_dir: &[String],
    include_ext: &[String],
    exclude_ext: &[String]
) -> Vec<PathBuf> {
    WalkDir::new(&dir).follow_links(follow_links).into_iter()
        .filter_entry(|d| !contains_dir(d, exclude_dir)) // ignore dirs (dir names) early if specified
        .filter_map(|result| if let Ok(entry) = result {
            match include(&entry, include_hidden, include_ext, exclude_ext) {
                true => {
                    if entry.path_is_symlink() && !follow_links {
                        None
                    } else {
                        Some(entry.path().to_owned())
                    }
                },
                    false => None,
                }
            } else {
                None
            }
        )
        .collect()
}

pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().map(|s| s.to_ascii_lowercase()) == Some(OsString::from(&ext.to_lowercase()))
}

pub fn acknowledge(msg: &str) -> std::io::Result<bool>{
    print!("{msg} (y/n): ");
    std::io::stdout().flush()?;
    let mut overwrite = String::new();
    std::io::stdin().read_line(&mut overwrite)?;

    loop {
        match overwrite.to_lowercase().trim_matches('\n') {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                println!("(!) Enter y/yes or n/no")
            }
        }
    }
}

/// Write file to disk, prompt user if target file exists
pub fn writefile(content: &String, outpath: &Path) -> std::io::Result<bool> {
    if Path::new(&outpath).exists() {
        let msg = format!("(!) '{}' already exists. Overwrite?", outpath.display());
        if acknowledge(&msg)? == false {
            return Ok(false)
        }
        // loop {
        //     print!("(!) '{}' already exists. Overwrite? (y/n): ", outpath.display());
        //     std::io::stdout().flush()?;
        //     let mut overwrite = String::new();
        //     std::io::stdin().read_line(&mut overwrite)?;

        //     match overwrite.to_lowercase().trim_matches('\n') {
        //         "y" | "yes" => break,
        //         "n" | "no" => return Ok(false),
        //         _ => {
        //             println!("(!) Enter y/yes or n/no");
        //             continue;
        //         }
        //     }
        // }
    }

    let mut outfile = File::create(&outpath)?;
    outfile.write_all(content.as_bytes())?;

    Ok(true)
}

/// Returns file count per extension as a hashmap: key: file extension, value: count.
/// `min_count` is the minimum file count that should be represented in the output,
///  e.g. if set to 3, any occurrence below 3 will be filtered out.
pub fn file_count(paths: &[PathBuf], min_count: Option<usize>, case_sensitive: bool) -> Vec<(std::string::String, usize)> {
    let mut extcount: HashMap<String, usize> = HashMap::new();

    for path in paths.iter() {
        if path.is_file() {
            match path.extension() {
                Some(ext) => {
                    match case_sensitive {
                        true => *extcount.entry(ext.to_string_lossy().to_string()).or_default() += 1,
                        false => *extcount.entry(ext.to_ascii_lowercase().to_string_lossy().to_string()).or_default() += 1
                    }
                },
                None => *extcount.entry(String::from("<NO FILE EXT>")).or_default() += 1,
            }
        }
    }

    let mut extsorted: Vec<(String, usize)> = extcount.iter()
        .map(|(e, i)| (e.to_owned(), *i))
        .collect();

    if let Some(min) = min_count {
        extsorted = extsorted.into_iter()
            .filter(|(_, size)| size > &(min - 1))
            .collect();
    }

    extsorted.sort_by_key(|(_, size)| *size);

    extsorted
}

pub fn filename_to_string(path: &Path) -> Option<String> {
    if let Some(filename) = path.file_name() {
        Some(filename.to_string_lossy().to_string())
    } else {
        None
    }
}

pub fn fileext_to_string(path: &Path) -> Option<String> {
    if let Some(fileext) = path.extension() {
        Some(fileext.to_string_lossy().to_string())
    } else {
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum LogLevel {
    Normal,
    // Verbose,
    None
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "normal" => Self::Normal,
            // "verbose" => Self::Verbose,
            _ => Self::None
        }
    }
}

impl From<bool> for LogLevel {
    fn from(value: bool) -> Self {
        match value {
            true => LogLevel::Normal,
            false => LogLevel::None,
        }
    }
}

/// Returns `(FILE_EXTENSION, SIZE_IN_BYTES, CREATED, MODIFIED)`.
/// 
/// Creation time is not available on all systems and is optional.
pub fn file_stats(path: &Path) -> std::io::Result<(Option<String>, u64, Option<OffsetDateTime>, OffsetDateTime)> {
    let metadata = path.metadata()?;

    // let ctime = systime2datetime(metadata.created()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // let mtime = systime2datetime(metadata.modified()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let ctime = FileTime::from_creation_time(&metadata).map(|ft| OffsetDateTime::UNIX_EPOCH + ft.unix_seconds().seconds());
    let mtime = OffsetDateTime::UNIX_EPOCH + FileTime::from_last_modification_time(&metadata).unix_seconds().seconds();

    Ok((
        fileext_to_string(path),
        metadata.len(),
        ctime,
        mtime
        // systime2datetime(metadata.created()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        // systime2datetime(metadata.modified()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    ))
}