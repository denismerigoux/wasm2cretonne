# wasm2cretonne

[Cretonne](https://github.com/stoklund/cretonne) frontend for WebAssembly. Reads wasm binary modules and translate the functions it contains into Cretonne IL functions.

## Example

```rust
use wasm2cretonne::module_translator::translate_module;
use std::path::Path;

let path = Path::new("tests/simple.wasm")
let data = match read_wasm_file(path) {
    Ok(data) => data,
    Err(err) => {
        println!("Error: {}", err);
    }
};
let funcs = match translate_module(&data) {
    Ok(funcs) => funcs,
    Err(string) => {
        println!(string);
    }
};
```

## Limitations

Runtime support is missing for now. Particularly:

* `set_global` and `get_global`;
* `grow_memory` and `current_memory`;
* tables for `call_indirect`.

These instructions are translated with placeholders.
