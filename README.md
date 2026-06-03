# ripit-rust

convenience wrapper for MakeMKV's command line utility (makemkvcon),
written in Rust

## Purpose

MakeMKV's command line utility is inconvenient to use for humans and for
scripting.

This repository provides 2 components:

- **makemkvcon**: A library which wraps makemkvcon with a nicer interface
  but doesn't change the underlying interface.
- **ripit-cli**: A command line application that provides its own user
  interface. It can be uses on its own or as a showcase how to use
  makemkvcon from your own Rust code.

> _Please note:_ This code was written by a seasoned programmer learning
> Rust. It combines a useful utility with a learning exercise. Expect to
> see unidiomatic code. Regardless of code quality, the aspiration is to
> have a usable application that is more convenient then makemkvcon.

## Versioning

This code uses [Semantic Versioning](https://semver.org) with the following
tweaks:

- If the change does not require changes to unit tests it's a `fix:`.
- If the change introduces a new unit tests it's a `feat:`.
- If the change modifies an existing unit tests it's a `feat!:`.

These tweaks are intended to make it obvious when a change would result in
a patch, a minor or a major release.

- Major releases could increase quickly. It's just a number, get over it.
- If untested code is changed it will never result in a minor or major
  release. This is intentional. The tests are the specification and
  documentation. The absence of test implies undefined behavior.

_Related:_ Refrain from using major versions for marketing purposes.
Assign a codename to highlight a specific major version.

## Maturity

### Code

| component | maturity level                      |
| --------- | ----------------------------------- |
| makemkv   | exploring, may change significantly |
| ripit-cli | exploring, may change significantly |

### Tooling

| component        | maturity level                 |
| ---------------- | ------------------------------ |
| dependabot       | untested                       |
| pre-commit hooks | working as desired, may change |
| release-please   | untested                       |
| (publishing)     | not implemented                |
