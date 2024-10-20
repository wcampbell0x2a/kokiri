# kokiri
Test revisions of crates against other revisions of crates.

## Example
Test master of [deku](https://github.com/sharksforarms/deku) against
other crates hosted on github.

### `instructions.toml`
```toml
[test]
url = "https://github.com/sharksforarms/deku"
name = "deku"
rev = "master"

[[instructions]]
# Required
url = "https://github.com/wcampbell0x2a/backhand.git"
# Required
name = "backhand"
# Optional
package = "backhand"
# Optional (master if omitted)
rev = "master"
# Optional cmd to take before test cmd
before_action = "cargo build --bins"
```

### Running
```
$ ./kokiri instructions.toml
```

## Usage
```
Usage: kokiri [OPTIONS] <CONFIG> [CMD]

Arguments:
  <CONFIG>  Config
  [CMD]     Command to run [default: check] [possible values: check, test]

Options:
      --root-dir <ROOT_DIR>
          Root directory, tmp if not given
      --from-github-dependents-info <FROM_GITHUB_DEPENDENTS_INFO>
          Github Dependents Json
      --no-exit-on-error
          Don't exit on single failure
      --no-stdout
          Don't emit stdout while running commands
  -h, --help
          Print help
```

## Using Github Dependents Info
Using [github-dependents-info](https://github.com/nvuillam/github-dependents-info), one
can test against all repos that github reports as a dependent.
```
$ github-dependents-info --repo sharksforarms/deku --json > out.json
$ ./kokiri instructions.toml check --from-github-dependents-info out.json --no-exit-on-error --root-dir tmp --no-stdout
````
