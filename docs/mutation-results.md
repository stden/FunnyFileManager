# Autocomplete mutation verification

2026-09-07, cargo-mutants 27.1.0, Windows, Rust 1.98.1.

```text
cargo mutants --no-config --file src/complete/matcher.rs --output target/mutations-autocomplete-verified -- --lib complete::
```

The first run found six surviving mutations. Independent scoring vectors were
added for consecutive matches, word boundaries and skipped characters.
The repeated run tested 31 mutations: **30 caught, one unviable, zero missed**.
It completed in approximately nine minutes including the clean baseline build.
Unviable means the mutation could not compile; it is not counted as caught.

This run covers the autocomplete matcher with completion library tests. It does
not establish a mutation score for the entire repository. Raw reports remain in
the ignored `target/` directory. Command completion timings and methodology are
in [autocomplete benchmarks](autocomplete-benchmarks.md).
