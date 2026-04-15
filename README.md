# Lyrics Program!

Simple lyrics program that you can use for something like
waybar.

Using [https://lrclib.net/] to request lyrics.

# Installation

Make sure you have `playerctl` in your `$PATH`.
(The package to install is usually just called `playerctl`).

## Cargo (git)

Install this program from git by running this command.

```sh
cargo install --git https://github.com/tblelrd/lyrical
```

Cargo will build the `lyrical` binary and place it in your `CARGO_INSTALL_ROOT`.
For more details on installation location see [the cargo book](https://doc.rust-lang.org/cargo/commands/cargo-install.html#description)

# Configuration

## Waybar

To have waybar show lyrics of the current song, you can create
a custom module for `lyrical`.

```json
"custom/lyrical": {
    "format": "{}",
    "exec": "$HOME/.cargo/bin/lyrical"
},
```

Then use the module by adding `"custom/lyrical"` to the module list.
