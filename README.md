# chksum
Compare file hierarchies, find duplicate files. Developed for personal use so there may be issues...

Compare folders `DIR1` and `DIR2`. Hash using Blake3, log results to file/s.
```
chksum --source-dir DIR1 --target-dir DIR2 --verbose --blake3 --log
```

Find duplicates for any kind of file, excluding JSON and Markdown files on the (macOS) desktop:
```
chksum --source-dir ~/Desktop --duplicates --exclude json md --verbose --blake3 --log
```

Find duplicates for Rust files:
```
chksum --source-dir ~/dev --duplicates --include rs --verbose --blake3 --log
```