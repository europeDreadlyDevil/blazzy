[![Blazzy Version](https://img.shields.io/crates/v/blazzy?label=blazzy)](https://crates.io/crates/blazzy)


## About

**Blazzy** is a very fast and lightweight file system observer server that works directly with the system API. *(for now
only for Windows)*

## How it works

```
blazzy -p "C:\\" -l -a -d "10:min" -c r
```

This command launches blazzy with viewing the entire directory, with auto-saving changes to a file every 10 minutes

## For show all flags

```
blazzy -h
```

## Getting current changes

```
curl 127.0.0.1:8080/get_cache
```

## Installation

### Cargo

```
cargo install blazzy
```

## License

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)
