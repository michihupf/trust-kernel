# tRust

A small x86_64 kernel written in Rust to learn about how kernels work.

## Building and Running
This project uses a [justfile](https://github.com/casey/just) to perform building and running the kernel in qemu.

| Operation | Command |
|-----------|---------|
| Build and run the kernel | `just run` |
| Build tests and run | `just test` |

Additionally features can be provided to change the kernel behaviour. Run `just features=verbose-vm run` to write every VGA buffer write to serial device as well. This is useful for debugging purposes.

The justfile allows for providing additional cargo parameters: `just run "--release"`.

## Debugging

The `gdb` and `debug` recipe is broken as of now. To debug the kernel run 

```
  cargo build
  just _debug <path/to/kernel.bin>
  gdb <path/to/kernel.bin> -ex "target remote :1234"
```
