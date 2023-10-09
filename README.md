# Lemmy-Stats-Crawler

Crawls Lemmy instances using nodeinfo and API endpoints, to generate a list of instances and overall details.

## Usage

lemmy-stats-crawler will discover new instances from other instances, but you have to seed it with a set of initial instances using the `--start-instances` argument.

```
cargo run -- --start-instances baraza.africa,lemmy.ml
```

For a complete list of arguments, use `--help`

```
cargo run -- --help
```
