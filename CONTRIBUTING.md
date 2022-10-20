# Contributing to dirty-debug

Thank you for your interest in contributing to dirty-debug.  We appreciate it!

If you are contributing with a pull request you might want to know about a few scripts:

* [`./tools/check.sh`](tools/check.sh) — Checks that everything is fine.  This includes checking that everything
  builds, the unit tests pass, and the code is correctly formatted.  If you need to format the code run
  `cargo fmt`.
* [`./tools/codecov.sh`](tools/codecov.sh) — Creates a code coverage report.  There is not a strict code coverage
  threshold, but we do want pretty much everything tested.
