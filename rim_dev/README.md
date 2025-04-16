# `RIM-dev`: Helper crate for RIM development

A development utility crate to simplify debugging & releasing procedures.

## Usage

Overview:

```console
Usage: cargo dev [OPTIONS] [COMMAND]

Options:
    -h, -help       Print this help message

Commands:
    dist, d         Generate release binaries
    run-manager     Run in manager mode
    vendor          Download packages for offline package build
    mock-rustup-server
                    Generate a mocked rustup dist server
```

### Debug manager (GUI)

```bash
cargo dev run-manager
```

### Debug manager (CLI)

```bash
cargo dev run-manager --cli
```

check for more manager-mode help

```bash
cargo dev run-manager --help
```

### Generate release binaries

```bash
cargo dev dist
```

### Other

for more functionalities, check `--help`

```bash
cargo dev --help
```
