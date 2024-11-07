2024-11-06

- BREAKING: Now defaults to Blake3. `--blake3` flag removed, `--sha256` flag added.
- NEW: Added `--ignore-path-errors`/`--ipe` (defaults to `false`) to ignore errors from compiling paths.
- NEW: Meaningless bars added to `stats`.
- Smaller internal fixes.
- KNWON ISSUE: Windows only. In (at least) powershell paths containing spaces that end in backslash will fail to parse and raise error. For now just remove the final backslash.