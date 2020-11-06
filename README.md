# srt2webvtt

Convert between srt and webvtt and apply a delta time. You can use as a CLI or
like a lib.

## CLI

```txt
USAGE:
    srt2webvtt [OPTIONS] [input [output]]

OPTIONS:
    -d, --delta <delta>                    The delta time to apply one subtitle [default: 0]
        --input-format <input-format>      The input subtitle format
        --output-format <output-format>    The output subtitle format
```

## Crate

Put in your `Cargo.toml`:

```ini
[dependencies]
srt2webvtt = { git = "https://github.com/HuguesGuilleus/srt2webvtt", version = "1.0"}
```
