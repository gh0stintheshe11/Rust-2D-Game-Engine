# Contributing

Thanks for your interest in the Rust 2D Game Engine! Bug reports, fixes,
documentation improvements and features are all welcome.

## Reporting bugs and proposing features

Open an issue using one of the templates. For bugs, include your OS, the
steps to reproduce, and what the editor console showed. For larger features,
please open an issue to discuss the design before writing a lot of code.

## Development setup

You need the Rust toolchain (1.95 or newer, see `rust-version` in
`Cargo.toml`). On Linux, install the system packages listed in the
[README](README.md#getting-started).

```bash
cargo run                       # the editor
cargo run -p hilltop_hop        # a demo game, standalone (also star_catcher)
cargo test --workspace          # all tests; headless, no window or audio needed
cargo bench --bench frame       # frame times for scenes of 100 to 2000 entities
```

Changes to code that runs every frame (scripts, physics, rendering) should
keep `cargo bench --bench frame` at least as fast; it runs headless, and
`cargo bench --bench frame -- sprites` runs the scenarios whose name contains
"sprites".

The repository is a cargo workspace: the engine library at the root (`src/`,
`tests/`), the editor in `editor/`, the demo games in `demo/`.
[docs/](docs/README.md) has a design document per module; start there.

## Making a change

Before opening a pull request, make sure that:

1. `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
   and `cargo fmt --all --check` pass (CI runs the same checks on Linux,
   Windows and macOS).
2. Bug fixes come with a test that fails without the fix, whenever the
   behavior can be tested headlessly. Editor UI code can be exercised with the
   headless tests in `editor/src/app/mod.rs`.
3. The module's document in `docs/` describes the new behavior, including its
   "Known limitations" section, and user-visible changes are listed under
   "Unreleased" in [CHANGELOG.md](CHANGELOG.md).

Code conventions are summarized in [AGENTS.md](AGENTS.md) (written for both
humans and coding agents): no `unsafe`, never panic on user data, report
errors through the console logger, and keep the engine free of editor code.

### Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary` in the imperative mood, with types such as `feat`,
`fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci` and `chore`. The body
explains what changed and why. Keep unrelated changes in separate commits.

## License

The project is licensed under the [Apache License 2.0](LICENSE). Unless you
explicitly state otherwise, any contribution you intentionally submit for
inclusion in the project is licensed under the same terms, without any
additional terms or conditions.
