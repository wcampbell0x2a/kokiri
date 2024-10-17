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
url = "https://github.com/wcampbell0x2a/backhand.git"
name = "backhand"
package = "backhand"
rev = "master"
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
  <CONFIG>
  [CMD]     [default: check] [possible values: check, test]

Options:
      --root-dir <ROOT_DIR>
  -h, --help                 Print help
```
