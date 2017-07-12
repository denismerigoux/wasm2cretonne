# wasm2cretonne

[Cretonne](https://github.com/stoklund/cretonne) frontend for WebAssembly. Reads wasm binary modules and translate the functions it contains into Cretonne IL functions.

## Example

```rust
use wasm2cretonne::translate_module;
use std::path::{Path, PathBuf};

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}

let path = Path::new("tests/simple.wasm");
let data = match read_wasm_file(path.to_path_buf()) {
    Ok(data) => data,
    Err(err) => {
        panic!("Error: {}", err);
    }
};
let funcs = match translate_module(&data) {
    Ok(funcs) => funcs,
    Err(string) => {
        panic!(string);
    }
};
```

## Limitations

Runtime support is missing for now. Particularly:

* `set_global` and `get_global`;
* `grow_memory` and `current_memory`;
* tables for `call_indirect`.

These instructions are translated with placeholders.
