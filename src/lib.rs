/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]
#![allow(clippy::inline_always)]
// Note: If you change this remember to update `README.md`.  To do so run `cargo rdme`.
//! `dirty-debug` offers a quick and easy way to log message to a file (or tcp endpoint) for
//! temporary debugging.
//!
//! A simple but powerful way to debug a program is to printing some messages to understand your
//! code’s behavior.  However, sometimes you don’t have access to the `stdout`/`stderr` streams (for
//! instance, when your code is loaded and executed by another program).  `dirty-debug` offers you a
//! simple, no-setup, way to log to a file:
//!
//! ```rust,no_run
//! # use dirty_debug::ddbg;
//! #
//! # let state = 42;
//! #
//! ddbg!("/tmp/debug_log", "Control reached here.  State={}", state);
//! ```
//!
//! It’s as simple as that.  Every time you call [`ddbg!()`](crate::ddbg) you will append the debug
//! message to that file, together with the filename and line number of the source code’s location.
//!
//! Note that this is not meant to be a normal form of logging: `dirty-debug` should only be used
//! temporarily during your debug session and discarded after that.
//!
//! # Logging to a TCP endpoint
//!
//! You can also use `dirty-debug` to log to a TCP endpoint instead of a file:
//!
//! ```rust,no_run
//! # use dirty_debug::ddbg;
//! #
//! # let state = 42;
//! #
//! ddbg!("tcp://192.168.1.42:12345", "Hello!");
//! ```
//!
//! Probably the easiest way to listen to a TCP endpoint in the target computer is by using netcat:
//!
//! ```console
//! $ ncat -l 12345
//! [src/lib.rs:123] Hello!
//! ```

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::str::FromStr;

static DIRTY_FILES: Lazy<DashMap<&str, File>> = Lazy::new(DashMap::new);

static DIRTY_TCP: Lazy<DashMap<(&str, u16), TcpStream>> = Lazy::new(DashMap::new);

/// Writes a message to the given location.  The message will be formatted.
///
/// # Example — Logging to a file
///
/// ```rust,no_run
/// # use dirty_debug::ddbg;
/// #
/// ddbg!("/tmp/log", "Hello {}!", "world");
/// ```
///
/// # Example — Logging to a tcp endpoint
///
/// ```rust,no_run
/// # use dirty_debug::ddbg;
/// #
/// ddbg!("tcp://192.168.1.42:12345", "Hello {}!", "world");
/// ```
#[macro_export]
macro_rules! ddbg {
    ($uri:expr, $f:literal) => {{
        $crate::dirty_log_message(
            $uri,
            ::std::format_args!(::std::concat!("[{}:{}] ", $f), ::std::file!(), ::std::line!()),
        );
    }};
    ($uri:expr, $f:literal, $($arg:tt)*) => {{
        $crate::dirty_log_message(
            $uri,
            ::std::format_args!(::std::concat!("[{}:{}] ", $f), ::std::file!(), ::std::line!(), $($arg)*),
        );
    }};
}

#[inline(always)]
fn dirty_log_str_writer(writer: &mut impl Write, args: fmt::Arguments<'_>) -> io::Result<()> {
    writer.write_fmt(args)?;
    writer.write_all("\n".as_bytes())?;

    // Performance won't be great if we flush all the time, but we don't want to lose log lines if
    // the program crashes.
    writer.flush()
}

#[inline(always)]
fn dirty_log_str_file(filepath: &'static str, args: fmt::Arguments<'_>) -> io::Result<()> {
    let mut entry = DIRTY_FILES.entry(filepath).or_try_insert_with(move || {
        let file = File::options().create(true).append(true).open(filepath)?;
        Ok::<_, io::Error>(file)
    })?;

    // `DashMap` ensures we have exclusive access to this file, so there is no way for two threads
    // to write to the same line.
    let file = entry.value_mut();

    dirty_log_str_writer(file, args)
}

#[inline(always)]
fn dirty_log_str_tcp(
    hostname: &'static str,
    port: u16,
    args: fmt::Arguments<'_>,
) -> io::Result<()> {
    let mut entry = DIRTY_TCP.entry((hostname, port)).or_try_insert_with(move || {
        let stream = TcpStream::connect((hostname, port))?;
        Ok::<_, io::Error>(stream)
    })?;

    // `DashMap` ensures we have exclusive access to this stream, so there is no way for two threads
    // to write to the same line.
    let stream = entry.value_mut();

    dirty_log_str_writer(stream, args)
}

/// Logs the given message.  The `uri` is a string with a static lifetime, so that it can be stored
/// without cloning, to avoid extra memory allocations.
#[doc(hidden)]
pub fn dirty_log_message(uri: &'static str, args: fmt::Arguments<'_>) {
    let result = if let Some(authority) = uri.strip_prefix("tcp://") {
        let (hostname, port) = authority.rsplit_once(':').expect("invalid tcp uri");

        // Ensure sure we can handle IPv6 uris like `tcp://[::1]:1234`:
        let hostname =
            hostname.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(hostname);
        let port = u16::from_str(port).expect("invalid port number");

        dirty_log_str_tcp(hostname, port, args)
    } else {
        let filepath = uri.strip_prefix("file://").unwrap_or(uri);

        dirty_log_str_file(filepath, args)
    };

    if let Err(e) = result {
        panic!("failed to log to \"{}\": {}", uri, e);
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;
    use std::collections::HashSet;
    use std::io::Read;
    use std::net::TcpStream;
    use std::thread::JoinHandle;

    struct TempFilepath {
        filepath: String,
    }

    impl TempFilepath {
        fn new() -> TempFilepath {
            use rand::distributions::Alphanumeric;
            use rand::thread_rng;
            use rand::Rng;

            let dir = std::env::temp_dir();
            let filename: String =
                thread_rng().sample_iter(&Alphanumeric).take(30).map(char::from).collect();

            let filepath = dir.join(format!("dirty_debug_test_{}", filename)).display().to_string();

            TempFilepath { filepath }
        }

        fn read(&self) -> String {
            std::fs::read_to_string(&self.filepath).unwrap()
        }
    }

    impl Drop for TempFilepath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.filepath);
        }
    }

    struct Listener {
        thread_handler: JoinHandle<String>,
        port: u16,
    }

    impl Listener {
        fn new() -> Listener {
            Listener::new_with_bind("127.0.0.1")
        }

        fn new_with_bind(bind: &str) -> Listener {
            use std::net::TcpListener;
            use std::thread::spawn;

            let listener: TcpListener =
                TcpListener::bind(format!("{}:0", bind)).expect("fail to bind");

            let port: u16 = listener.local_addr().unwrap().port();

            let thread_handler = spawn(move || {
                let mut content: String = String::with_capacity(1024);
                let mut stream: TcpStream = listener.incoming().next().unwrap().unwrap();

                while !content.contains("==EOF==") {
                    let mut buffer: [u8; 8] = [0; 8];
                    let read = stream.read(&mut buffer).unwrap();
                    let s = std::str::from_utf8(&buffer[0..read]).unwrap();
                    content.push_str(s);
                }

                content
            });

            Listener { thread_handler, port }
        }

        fn content(self) -> String {
            self.thread_handler.join().unwrap()
        }
    }

    /// Creates a `&'static str` out of any string.  This is important because the uri in `ddbg!()`
    /// needs to be a string with a static lifetime to allow it to be stored without cloning it.
    macro_rules! make_static {
        ($str:expr) => {{
            static CELL: ::once_cell::sync::OnceCell<String> = ::once_cell::sync::OnceCell::new();
            CELL.set($str.to_owned()).unwrap();
            CELL.get().unwrap().as_str()
        }};
    }

    fn read_log_strip_source_info(log: String) -> String {
        let mut stripped_log = String::with_capacity(log.len());

        for line in log.lines() {
            let stripped = match line.starts_with('[') {
                true => line.splitn(2, " ").skip(1).next().unwrap_or(""),
                false => line,
            };

            stripped_log.push_str(stripped);
            stripped_log.push('\n');
        }

        stripped_log
    }

    fn assert_log(log: String, expected: &str) {
        let stripped_log = read_log_strip_source_info(log);

        assert_eq!(stripped_log, expected);
    }

    #[test]
    fn test_ddbg_file_and_line_number() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);

        ddbg!(filepath, "test");
        let line = line!() - 1;

        assert_eq!(temp_file.read(), format!("[{}:{}] test\n", file!(), line));
    }

    #[test]
    fn test_ddbg_simple() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);

        ddbg!(filepath, "numbers={:?}", [1, 2, 3]);

        assert_log(temp_file.read(), "numbers=[1, 2, 3]\n");
    }

    #[test]
    fn test_ddbg_multiple_syntaxes() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);

        ddbg!(filepath, "nothing to format");
        ddbg!(filepath, "another nothing to format",);
        ddbg!(filepath, "");
        ddbg!(filepath, "a {} b {}", 23, "foo");
        ddbg!(filepath, "a {} b {}", 32, "bar",);

        let expected = indoc! { r#"
            nothing to format
            another nothing to format

            a 23 b foo
            a 32 b bar
            "#
        };

        assert_log(temp_file.read(), expected);
    }

    #[test]
    fn test_ddbg_file_append() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);

        std::fs::write(filepath, "[file.rs:23] first\n").unwrap();

        ddbg!(filepath, "second");

        let expected = indoc! { r#"
            first
            second
            "#
        };

        assert_log(temp_file.read(), expected);
    }

    #[test]
    fn test_ddbg_multiline() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);

        ddbg!(filepath, "This log\nmessage\nspans multiple lines!");

        let expected = indoc! { r#"
            This log
            message
            spans multiple lines!
            "#
        };

        assert_log(temp_file.read(), expected);
    }

    #[test]
    fn test_ddbg_uri_scheme_file() {
        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(format!("file://{}", temp_file.filepath));

        ddbg!(filepath, "test!");

        assert_log(temp_file.read(), "test!\n");
    }

    #[test]
    fn test_ddbg_multithread_no_corrupted_lines() {
        use std::str::FromStr;
        use std::thread::{spawn, JoinHandle};

        const THREAD_NUM: usize = 20;
        const ITERATIONS: usize = 1000;
        const REPETITIONS: usize = 1000;

        let temp_file: TempFilepath = TempFilepath::new();
        let filepath: &'static str = make_static!(temp_file.filepath);
        let mut threads: Vec<JoinHandle<()>> = Vec::with_capacity(THREAD_NUM);

        for i in 0..THREAD_NUM {
            let thread = spawn(move || {
                for j in 0..ITERATIONS {
                    ddbg!(filepath, "{}", format!("{}:{}_", i, j).repeat(REPETITIONS));
                }
            });

            threads.push(thread);
        }

        for thread in threads {
            thread.join().unwrap();
        }

        let mut lines_added: HashSet<(u16, u16)> = HashSet::with_capacity(THREAD_NUM * ITERATIONS);

        for i in 0..THREAD_NUM {
            for j in 0..ITERATIONS {
                lines_added.insert((i as u16, j as u16));
            }
        }

        let log = read_log_strip_source_info(temp_file.read());

        for line in log.lines() {
            let token = line.splitn(2, "_").next().unwrap();
            let mut iter = token.split(":");
            let i = u16::from_str(iter.next().unwrap()).unwrap();
            let j = u16::from_str(iter.next().unwrap()).unwrap();
            let expected = format!("{}:{}_", i, j).repeat(REPETITIONS);

            assert_eq!(line, expected);

            lines_added.remove(&(i, j));
        }

        assert!(lines_added.is_empty());
    }

    #[test]
    fn test_ddbg_uri_scheme_tcp_hostname() {
        let tcp_listener: Listener = Listener::new();
        let uri: &'static str = make_static!(format!("tcp://localhost:{}", tcp_listener.port));

        ddbg!(uri, "test hostname!");
        ddbg!(uri, "==EOF==");

        assert_log(tcp_listener.content(), "test hostname!\n==EOF==\n");
    }

    #[test]
    fn test_ddbg_uri_scheme_tcp_ipv4() {
        let tcp_listener: Listener = Listener::new();
        let uri: &'static str = make_static!(format!("tcp://127.0.0.1:{}", tcp_listener.port));

        ddbg!(uri, "test ipv4!");
        ddbg!(uri, "==EOF==");

        assert_log(tcp_listener.content(), "test ipv4!\n==EOF==\n");
    }

    #[test]
    fn test_ddbg_uri_scheme_tcp_ipv6() {
        let tcp_listener: Listener = Listener::new_with_bind("::1");
        let uri: &'static str = make_static!(format!("tcp://[::1]:{}", tcp_listener.port));

        ddbg!(uri, "test ipv6!");
        ddbg!(uri, "==EOF==");

        assert_log(tcp_listener.content(), "test ipv6!\n==EOF==\n");
    }
}
