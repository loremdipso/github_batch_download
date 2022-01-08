Simple utility to download public github repos.

Build:
```
cargo build
```

Example usage:
```
cargo run -- --language svelte --license mit --output ./output --limit 100
```

This will download up to `100` repositories with an `MIT` license and with `svelte` listed as one of their languages to the `output` directory.
