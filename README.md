# feaders-rs
[![Clippy Linting Result](https://clippy.bashy.io/github/shaded-enmity/feaders-rs/master/badge.svg)](https://clippy.bashy.io/github/shaded-enmity/feaders-rs/master/log) [![Crates.io](https://img.shields.io/crates/v/feaders.svg?maxAge=2592000)](Cargo.toml) [![Crates.io](https://img.shields.io/crates/l/feaders.svg?maxAge=2592000)](LICENSE) [![GitHub issues](https://img.shields.io/github/issues/shaded-enmity/feaders-rs.svg)](https://github.com/shaded-enmity/feaders-rs/issues)

Reimplementation of the [Feaders](https://github.com/shaded-enmity/feaders) project in Rust.

# Usage
```
$ feaders -h
Usage: feaders [options] PATH

Options:
    -h, --help          prints this menu
    -r, --repo          repository to use for resolution
    -v, --verbose       verbose mode
    -d, --deduplicate   try to deduplicate headers
        --version       display version information
```
(*note: the repository parameter is currently ignored and all repositories found in `/etc/yum.repos.d/` are searched*)

Example run:
```
$ feaders -d ../Pillow/
python-devel-2.7.10-8.fc22.x86_64
tk-devel-1:8.6.4-2.fc22.x86_64
libjpeg-turbo-devel-1.4.0-2.fc22.x86_64
libtiff-devel-4.0.3-21.fc22.x86_64
openjpeg-devel-1.5.1-14.fc22.x86_64
zlib-devel-1.2.8-7.fc22.x86_64
lcms2-devel-2.7-1.fc22.x86_64
libwebp-devel-0.4.4-1.fc22.x86_64
```

# Differences from original Feaders
Aside from being written in Rust instead of Python, `feaders-rs` takes a different stab at searching repositories. The original implementation relied on `librepo` to download `sqlite` representation of repositories and split the workflow into a client/server part, where the client was used to search the file system for C/C++ files, extract `#include` statements and query the server which performed all the `sqlite` queries. 

`feaders-rs` uses `libhif` to faciliate the search via `libsolv` queries. FFI definitions for `libhif` were auto generated using [rust-bindgen](https://github.com/crabtw/rust-bindgen) and the Dockerized generator is available in it's own repository [rust-libhif](https://github.com/shaded-enmity/rust-libhif).

# Notes
I've used this project to actually learn Rust on the go, if you catch a silly mistake or just "bad Rust" please open an issue or send a pull request.

# License
GPL-3.0
