# ComfyUI Rust Filelist

This custom node replaces the model folder file scan with a Rust implementation.
If the Rust extension is not available, it falls back to the original Python scan.

## Supported Python and platforms
- Python 3.10 to 3.12
- Windows x64
- macOS arm64
- Linux x64

## How it works
- On import, it patches `folder_paths.get_filename_list_`.
- It calls the Rust extension module `comfyui_rust_filelist`.
- If the module is missing, it keeps the Python implementation.

## Build locally
1) Install Rust and maturin
2) Build and install into your Python environment

```bash
python -m pip install maturin
maturin develop --release --manifest-path rust/Cargo.toml
```

## Using prebuilt binaries
Place the built extension file in this folder so Python can import it.
The file name should match `comfyui_rust_filelist` with the platform suffix:
- Windows: `comfyui_rust_filelist.pyd`
- macOS: `comfyui_rust_filelist.so`
- Linux: `comfyui_rust_filelist.so`
