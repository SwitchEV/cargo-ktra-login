# cargo-ktra-login

A Cargo subcommand that logs you in to your little cargo registry.

## What is this for?

Given a username and a password, this Cargo subcommand will perform the following steps:

- Identify the registry for the current package
- Check out the registry's repository
- Use the [ktra login API](https://book.ktra.dev/ktra_web_apis.html) to generate a token
- Add the token to Cargo

## Usage

cargo-ktra-login has the following command signature:
```
Log in to private ktra registries using cargo

Usage: cargo ktra-login [OPTIONS] <USERNAME> <PASSWORD>

Arguments:
  <USERNAME>  The user account to log in with
  <PASSWORD>  The account password to log in with

Options:
      --manifest-path <PATH>  Path to Cargo.toml
      --dry-run               Check that the manifest is valid and that the remote registry exists, but don't generate a token
      --registry <REGISTRY>   The name of the registry to log in to
  -h, --help                  Print help
  -V, --version               Print version
```
