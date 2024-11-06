use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use filetime::FileTime;
use time::{ext::NumericalDuration, OffsetDateTime};
use walkdir::{DirEntry, WalkDir};

/// Passed to `Walkdir::filter`. Returns `true` if `DirEntry`:
/// - is/is not hidden, (`include_hidden` - currently only Unix/Linux)
/// - does not contains a component/dir name that equeals `ignore_dir`
/// - has a file extension specified in `include_ext`
/// - does no have a file extension specified in `exclude_ext`
pub fn include(
    direntry: &DirEntry,
    include_hidden: bool,
    follow_links: bool,
    include_ext: &[String],
    exclude_ext: &[String],
    exclude_dir: &[String],
) -> bool {

    // WalkDir should have already followed/resolved
    // paths if follow_links is set, but when not set
    // symlinks need to be ignored
    if direntry.path_is_symlink() && !follow_links {
        return false;
    }

    if direntry.path().is_dir() {
        return false;
    }

    if contains_dir(direntry, exclude_dir) {
        return false;
    }

    let is_hidden = direntry
        .file_name()
        .to_str()
        .map(|n| n.starts_with(".")) // unix/linux only, windows has attributes that need checking
        .unwrap_or(false);
    if !include_hidden && is_hidden {
        return false;
    }

    if let Some(ext) = direntry.path().extension() {
        let ext_str = ext.to_string_lossy().to_ascii_lowercase();
        if !include_ext.is_empty() {
            return include_ext.contains(&ext_str);
        }

        if !exclude_ext.is_empty() {
            return !exclude_ext.contains(&ext_str);
        }
    } else {
        // Check to ignore files without extension whenever
        // include extentions are specified
        if !include_ext.is_empty() {
            return false;
        }
    }

    true
}

/// Passed to `Walkdir::filter`. Returns `Some(PATH)` if `DirEntry`:
/// - is/is not hidden, (`include_hidden` - currently only Unix/Linux)
/// - does not contains a component/dir name that equeals `ignore_dir`
/// - has a file extension specified in `include_ext`
/// - does not have a file extension specified in `exclude_ext`
pub fn include2(
    direntry: &DirEntry,
    include_hidden: bool,
    follow_links: bool,
    include_ext: &[String],
    exclude_ext: &[String],
    exclude_dir: &[String],
// ) -> bool {
) -> Option<PathBuf> {

    // WalkDir should have already followed/resolved
    // paths if follow_links is set, but when not set
    // symlinks need to be ignored
    if direntry.path_is_symlink() && !follow_links {
        return None
    }

    if direntry.path().is_dir() {
        return None
    }

    if contains_dir(direntry, exclude_dir) {
        return None
    }

    let is_hidden = direntry
        .file_name()
        .to_str()
        .map(|n| n.starts_with(".")) // unix/linux only, windows has attributes that need checking
        .unwrap_or(false);
    if !include_hidden && is_hidden {
        return None
    }

    if let Some(ext) = direntry.path().extension() {
        let ext_str = ext.to_string_lossy().to_ascii_lowercase();
        if !include_ext.is_empty() {
            if include_ext.contains(&ext_str) {
                return Some(direntry.path().to_owned());
            }
        }

        if !exclude_ext.is_empty() {
            if !exclude_ext.contains(&ext_str) {
                return Some(direntry.path().to_owned());
            }
        }
    } else {
        // Check to ignore files without extension whenever
        // include extentions are specified
        if !include_ext.is_empty() {
            return None
        }
    }

    Some(direntry.path().to_owned())
}

fn contains_dir(direntry: &DirEntry, dirs: &[String]) -> bool {
    for dir in dirs.iter() {
        if direntry
            .path()
            .components()
            .any(|c| c == std::path::Component::Normal(OsStr::new(dir)))
        {
            return true;
        }
    }
    false
}

/// Compile paths, but halts on errors.
pub fn paths(
    dir: &Path,
    include_hidden: bool,
    follow_links: bool,
    exclude_dir: &[String],
    include_ext: &[String],
    exclude_ext: &[String],
    ignore_error: bool
) -> std::io::Result<Vec<PathBuf>> {
    WalkDir::new(&dir)
        // if follow symlinks = true, WalkDir must yield the followed path
        // or chksum will attempt to open the symlink instead of the target
        // path which will raise an error
        .follow_links(follow_links)
        .into_iter()
        .filter_map(|direntry|
            // Filter out dirs/dirnames/extensions
            match direntry {
                Ok(entry) => {
                    // include2(&entry, include_hidden, follow_links, include_ext, exclude_ext, exclude_dir)
                    if include(&entry, include_hidden, follow_links, include_ext, exclude_ext, exclude_dir) {
                        Some(Ok(entry.path().to_owned()))
                    } else {
                        None
                    }
                },
                Err(e) => {
                    // Whether to continue on error or not
                    // E.g. when a socket is encountered
                    match ignore_error {
                        true => None,
                        false => Some(Err(e.into()))
                    }
                },
            }
                // must check follow links again here, since if not set,
                // the link will raise error later on
        )
        .collect::<Result<Vec<_>, _>>()
}

pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().map(|s| s.to_ascii_lowercase()) == Some(OsString::from(&ext.to_lowercase()))
}

pub fn has_extension_any(path: &Path, exts: &[&str]) -> bool {
    exts.iter().any(|ext| has_extension(path, ext))
}

pub fn confirm(msg: &str) -> std::io::Result<bool> {
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
    if outpath.exists() {
        let msg = format!("(!) '{}' already exists. Overwrite?", outpath.display());
        if confirm(&msg)? == false {
            return Ok(false);
        }
    }

    let mut outfile = File::create(&outpath)?;
    outfile.write_all(content.as_bytes())?;

    Ok(true)
}

/// Returns file count per extension as a hashmap: key: file extension, value: count.
/// `min_count` is the minimum file count that should be represented in the output,
///  e.g. if set to 3, any occurrence below 3 will be filtered out.
pub fn file_count(
    paths: &[PathBuf],
    min_count: Option<usize>,
    case_sensitive: bool,
) -> Vec<(std::string::String, usize)> {
    let mut extcount: HashMap<String, usize> = HashMap::new();

    for path in paths.iter() {
        if path.is_file() {
            match path.extension() {
                Some(ext) => match case_sensitive {
                    true => {
                        *extcount
                            .entry(ext.to_string_lossy().to_string())
                            .or_default() += 1
                    }
                    false => {
                        *extcount
                            .entry(ext.to_ascii_lowercase().to_string_lossy().to_string())
                            .or_default() += 1
                    }
                },
                None => *extcount.entry(String::from("<NO FILE EXT>")).or_default() += 1,
            }
        }
    }

    let mut extsorted: Vec<(String, usize)> =
        extcount.iter().map(|(e, i)| (e.to_owned(), *i)).collect();

    if let Some(min) = min_count {
        extsorted = extsorted
            .into_iter()
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
    None,
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "normal" => Self::Normal,
            // "verbose" => Self::Verbose,
            _ => Self::None,
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
pub fn file_stats(
    path: &Path,
) -> std::io::Result<(Option<String>, u64, Option<OffsetDateTime>, OffsetDateTime)> {
    let metadata = path.metadata()?;

    // let ctime = systime2datetime(metadata.created()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // let mtime = systime2datetime(metadata.modified()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let ctime = FileTime::from_creation_time(&metadata)
        .map(|ft| OffsetDateTime::UNIX_EPOCH + ft.unix_seconds().seconds());
    let mtime = OffsetDateTime::UNIX_EPOCH
        + FileTime::from_last_modification_time(&metadata)
            .unix_seconds()
            .seconds();

    Ok((
        fileext_to_string(path),
        metadata.len(),
        ctime,
        mtime, // systime2datetime(metadata.created()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
               // systime2datetime(metadata.modified()?).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    ))
}

pub fn open_file(path: &Path) -> std::io::Result<Option<File>> {
    match std::fs::File::open(path) {
        Ok(f) => Ok(Some(f)),
        Err(err) => match err.kind() {
            // attempt to ignore operations on sockets on *nix
            std::io::ErrorKind::Other => {
                if let Some(os_err) = err.raw_os_error() {
                    if os_err == 102 {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                } else {
                    Err(err)
                }
            }
            _ => Err(err),
        },
    }
}
