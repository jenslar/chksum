use std::{collections::HashMap, path::{PathBuf, Path}, fs::File, io::{Cursor, Read}};
use std::io::copy;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::{datetime::datetime_to_string, errors::ChksumError};

/// Hash files. Optionally, limit how many bytes to hash via `len`.
/// Returns hashmap with key: `<RELATIVE_PATH>`, value: `(<FULL_PATH, HASH>)`.
pub fn hash_files(
    paths: &[PathBuf],
    dirtype: &str,
    hashtype: &HashType,
    verbose: bool,
    len: Option<usize>,
    strip_prefix: Option<&Path>
) -> Result<HashMap<PathBuf, (PathBuf, String)>, ChksumError> {
    Ok(paths.par_iter()
        .map(|full_path| {
            if !full_path.exists() {
                return Err(ChksumError::FileDoesNotExist(full_path.to_owned())) // or just continue?
            }

            let timestamp_in = OffsetDateTime::now_utc();
            // let (hash, size) = hash_file(full_path, hashtype, len).expect("Failed to hash file");
            let (hash, size) = hash_file(full_path, hashtype, len)?;
            let timestamp_out = OffsetDateTime::now_utc();
            
            // Convert hash in bytes to hex string
            let hex_string = hash
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("");

            // forgot to pad hex string with zeros earlier so crappy len check
            assert_eq!(hex_string.len(), 64, "File hash has unexpected length");

            if verbose {
                // if print is not a single statement its print order
                // is not consistent (expected rayon behaviour or bug? since this is inside a closure)
                println!(
                    "─ {} ┬ FILE {:23} {}\n         └ HASH {:23} {} {:12} bytes",
                    dirtype,
                    datetime_to_string(&timestamp_in),
                    full_path.display(),
                    datetime_to_string(&timestamp_out),
                    hex_string,
                    size
                );
            }

            let stripped_path = if let Some(prefix) = strip_prefix {
                full_path.strip_prefix(prefix)?.to_owned()
            } else {
                full_path.to_owned()
            };

            // not using hash as key since duplicate file hashes will be discarded
            // whereas relative path will be unique
            Ok((stripped_path, (full_path.to_owned(), hex_string)))
        })
        .collect::<Result<HashMap<PathBuf, (PathBuf, String)>, ChksumError>>()?)
}

/// Organise hashes as `<KEY: hash, VAL: [sorted paths]>` to group duplicate files.
pub fn hash2path(hashes: &HashMap<PathBuf, (PathBuf, String)>) -> HashMap<String, Vec<PathBuf>> {
    let mut hash2paths: HashMap<String, Vec<PathBuf>> = HashMap::new();
    hashes.iter()
        .for_each(|(_path, (full_path, hash))| {
            let entry = hash2paths.entry(hash.to_owned());
            entry.or_default().push(full_path.to_owned())
        });
        
    hash2paths.iter_mut()
        .for_each(|(_, val)| val.sort());

    hash2paths
}

#[derive(Debug, Clone)]
pub enum HashType {
    Sha256,
    Blake3
}

impl HashType {
    pub fn to_string(&self) -> String {
        match &self {
            HashType::Blake3 => "BLAKE3".to_owned(),
            HashType::Sha256 => "SHA256".to_owned()
        }
    }
}

/// Calculates hash, and returns `(hash_as_bytes, bytes_read)`. 
pub fn hash_reader<R: Read>(reader: &mut R, hashtype: &HashType) -> std::io::Result<(Vec<u8>, u64)> {
    let hash: Vec<u8>;
    let size: u64;

    match hashtype {
        &HashType::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            size = copy(reader, &mut hasher)?;
            hash = hasher.finalize().as_bytes().to_ascii_lowercase()
        },
        &HashType::Sha256 => {
            let mut hasher = Sha256::new();
            size = copy(reader, &mut hasher)?;
            hash = hasher.finalize().to_ascii_lowercase();
        }
    }

    Ok((hash, size))
}

/// Hashes file, and returns `(hash_as_bytes, bytes_read)`.
pub fn hash_file(path: &Path, hashtype: &HashType, len: Option<usize>) -> Result<(Vec<u8>, u64), ChksumError> {
    if let Some(l) = len {
        let mut buf: Vec<u8> = vec![0; l];
        // Not important wether n == len here, i.e. file smaller than n bytes
        // just need input for hashing bytes for quick duplicate file elimination.
        // Added custom error that forwards path that failed, since sockets are traversed
        // like files and raise error when attempting to open.
        let _len = File::open(path)
            .map_err(|err| ChksumError::OpenFileFailed((path.to_owned(), err)))?
            .read(&mut buf)
            .map_err(|err| ChksumError::ReadFileFailed((path.to_owned(), err)))?;

        hash_reader(&mut Cursor::new(&buf), hashtype)
            .map_err(|err| ChksumError::PartialHashFailed((path.to_owned(), err)))
    } else {
        let mut file = File::open(path)
            .map_err(|err| ChksumError::OpenFileFailed((path.to_owned(), err)))?;
        hash_reader(&mut file, hashtype)
            .map_err(|err| ChksumError::HashFailed((path.to_owned(), err)))
    }
}
