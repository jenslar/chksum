# chksum
Compare file hierarchies, find duplicate files.

Compare folders `DIR1` and `DIR2`. Hash using Blake3, log results to file/s.
```
chksum --source-dir DIR1 --target-dir DIR2 --verbose --blake3 --log
```

Find duplicates for any kind of file, excluding JSON and Markdown files on the (macOS) desktop:
```
chksum --source-dir ~/Desktop --duplicates --exclude-ext json md --verbose --blake3 --log
```

Find duplicates for Rust files:
```
chksum --source-dir ~/dev --duplicates --include-ext rs --verbose --blake3 --log
```

List relative total size for each file type (extension) encountered in `source-dir:
```
chksum --source-dir ~/Desktop --stats                               # default lists size above 1.0% of total, change this with 'threshold'
chksum --source-dir ~/Desktop --stats --threshold 0.0               # list any file type with relative total size above 0% (i.e. all)
chksum --source-dir ~/Desktop --stats --include-ext xlsx ods csv    # only consider Excel, Libre Office Calc, and CSV-files
```
