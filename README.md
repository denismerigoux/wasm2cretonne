# wasm2cretonne

[Cretonne](https://github.com/stoklund/cretonne) frontend for WebAssembly. Reads wasm binary modules and translate the functions it contains into Cretonne IL functions.

The translation needs some info about the runtime in order to handle the wasm instructions `get_global`, `set_global`, and `call_indirect`. These informations are included in structs implementing the `WasmRuntime` trait like `DummyRuntime` or `StandaloneRuntime`.

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
let mut runtime = StandaloneRuntime::new();
let funcs = match translate_module(&data, &mut runtime) {
    Ok(funcs) => funcs,
    Err(string) => {
        panic!(string);
    }
};
```

## Standalone runtime specification

The `StandaloneRuntime` is a setup for in-memory execution of the module being translated. It allocates memory for the wasm linear memories, the globals and the tables and embeds the addresses of these memories inside the generated Cretonne IL functions.
