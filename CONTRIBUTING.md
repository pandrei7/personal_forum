# Contributing guide for Personal forum

Thank you for showing interest in this project! Contributions of most kinds are
welcome: from code, to bug reports, to documentation. This guide mentions some
things you should keep in mind if you want to contribute.

This project is hosted on GitHub, which is also used to talk about issues and
feature requests, and to merge pull requests. In general, if you want to talk
about something related to this project,
[opening an issue](https://docs.github.com/en/free-pro-team@latest/github/managing-your-work-on-github/creating-an-issue)
should be fine.

## Reporting issues

If you've encountered a bug, please create an issue describing it. You don't
have to follow a strict template, but these are some things you should consider
writing:

- a general description of the bug
- some steps to reproduce the issue
- what you think should happen normally
- images, if relevant (e.g., if you are reporting a visual bug).

## Contributing code or documentation

### How to add your changes

1. [Fork this repository](https://guides.github.com/activities/forking/).
2. Create a new branch off of **`develop`**.
3. Commit your changes to your new branch.
4. [Make a pull request](https://guides.github.com/activities/forking/#making-changes)
    and describe your changes.

### Coding style

For the backend code, we use Rust's standard coding style tools, like
[`rustfmt`](https://github.com/rust-lang/rustfmt).
If you modify some Rust code, make sure to run `cargo fmt` to format the
project. Regardless of language, try to be consistent with the existing code.

### Documentation

Most of this project's code should be documented. If you add new code, remember
to document at least its "public" parts. If you modify existing code, try to
also update the documentation, both in the source files, and in the [docs](docs)
directory.

## Proposing a feature

If you think a new feature is needed, simply create an issue describing your
idea and why you think it's useful. The discussion will continue inside that
issue.

## License

By contributing, you agree that you contributions will be licensed under this
project's [MIT license](LICENSE).
