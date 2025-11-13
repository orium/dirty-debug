[![Build Status](https://github.com/orium/dirty-debug/workflows/CI/badge.svg)](https://github.com/orium/dirty-debug/actions?query=workflow%3ACI)
[![Code Coverage](https://codecov.io/gh/orium/dirty-debug/branch/main/graph/badge.svg)](https://codecov.io/gh/orium/dirty-debug)
[![Dependency status](https://deps.rs/repo/github/orium/dirty-debug/status.svg)](https://deps.rs/repo/github/orium/dirty-debug)
[![crates.io](https://img.shields.io/crates/v/dirty-debug.svg)](https://crates.io/crates/dirty-debug)
[![Downloads](https://img.shields.io/crates/d/dirty-debug.svg)](https://crates.io/crates/dirty-debug)
[![Github stars](https://img.shields.io/github/stars/orium/dirty-debug.svg?logo=github)](https://github.com/orium/dirty-debug/stargazers)
[![Documentation](https://docs.rs/dirty-debug/badge.svg)](https://docs.rs/dirty-debug/)
[![License](https://img.shields.io/crates/l/dirty-debug.svg)](./LICENSE.md)

# Dirty debug

<!-- cargo-rdme start -->

`dirty-debug` offers a quick and easy way to log message to a file (or tcp endpoint) for
temporary debugging.

A simple but powerful way to debug a program is to printing some messages to understand your
code’s behavior. However, sometimes you don’t have access to the `stdout`/`stderr` streams (for
instance, when your code is loaded and executed by another program). `dirty-debug` offers you a
simple, no-setup, way to log to a file:

```rust
ddbg!("/tmp/debug_log", "Control reached here. State={}", state);
```

It’s as simple as that. Every time you call [`ddbg!()`](https://docs.rs/dirty-debug/latest/dirty_debug/macro.ddbg.html) you will append the debug
message to that file, together with the filename and line number of the source code’s location.

Note that this is not meant to be a normal form of logging: `dirty-debug` should only be used
temporarily during your debug session and discarded after that.

## Logging to a TCP endpoint

You can also use `dirty-debug` to log to a TCP endpoint instead of a file:

```rust
ddbg!("tcp://192.168.1.42:12345", "Hello!");
```

Probably the easiest way to listen to a TCP endpoint in the target computer is by using netcat:

```console
$ ncat -l 12345
[src/lib.rs:123] Hello!
```

<!-- cargo-rdme end -->
