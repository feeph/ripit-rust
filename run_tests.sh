#!/bin/bash
#
# perform unit tests and measure code coverage using cargo-llvm-cov
# <https://github.com/taiki-e/cargo-llvm-cov>
#
# VS Code:
#  - install extension 'wequick.coverage-gutters'
#  - enable coverage gutters

set -e
set -u

if ! $(cargo llvm-cov --version >/dev/null 2>&1) ; then
    echo "E: Unable to find 'cargo-llvm-cov'!" >&2
    echo "E: Please run \`cargo install cargo-llvm-cov\`." >&2
    exit 1
fi

for cargo_toml in */Cargo.toml ; do
    # normal unit testing (without code coverage)
    # cargo test --manifest-path=$cargo_toml

    # perform unit tests and measure code coverage
    # (requires `cargo install cargo-llvm-cov`)
    lcov_file=$(echo $cargo_toml|sed 's|/Cargo\.toml$|/lcov.info|')
    cargo llvm-cov --manifest-path=$cargo_toml --lcov --output-path=$lcov_file
done
