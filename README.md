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

**NOTE**: github is rate limited. If you authenticate you can make more requests before hitting this limit. To authenticate set your `GITHUB_TOKEN` environment variable to your [personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token). You can also create a `.env` with the following format:

```
GITHUB_TOKEN=...
```
