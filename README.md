# Simulink XML System Parser (Rust)

This crate parses Simulink `.xml` system descriptions (extracted from `.slx`) into a Rust data model and can output JSON.

- Recursively resolves `<System Ref="..."/>` and parses nested systems.
- Captures system properties, blocks (including ports and block properties), and lines with branches.

## Quick start

- Build:

```sh
cargo build
```

- Run against your workspace root system:

```sh
cargo run -- ../simulink/systems/system_root.xml > parsed.json
```

If you run without an argument, it will try `simulink/systems/system_root.xml` relative to the current directory.

## Examples

Print an ASCII tree of SubSystems in a model (works with `.slx` or individual XML):

```sh
cargo run --example tree -- ASXTest.slx
```

Or point to an XML system file:

```sh
cargo run --example tree -- simulink/systems/system_root.xml
```

## Library usage

```rust
use rustylink::parser::SimulinkParser;
use camino::Utf8PathBuf;

let parser = SimulinkParser::new(".");
let system = parser.parse_system_file(Utf8PathBuf::from("simulink/systems/system_root.xml"))?;
println!("{}", serde_json::to_string_pretty(&system)?);
```

## Notes

- The data model is intentionally generic (maps for properties) to accommodate varying Simulink versions.
- Extend `model.rs` to add more explicit types for blocks you care about.
