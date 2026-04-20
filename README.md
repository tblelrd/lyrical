# Lyrics Program!

[![crates.io](https://img.shields.io/crates/v/lyrical)](https://crates.io/crates/lyrical)
[![licence](https://img.shields.io/crates/l/lyrical)](https://github.com/tblelrd/lyrical)

Simple lyrics program that you can use for something like
waybar.

Using [lrclib.net](https://lrclib.net/) to request lyrics.

Automatically romanizes chinese, japanese, and korean lyrics.
Will develop a toggle for it in the future.

No AI was used for the creation of this program!

Example of this program on waybar.
![lyrics-on-waybar](./lyrics-on-waybar.png)

# Installation

Make sure you have `playerctl` in your `$PATH`.
(The package to install is usually just called `playerctl`).

## Cargo (crates.io)

Install the program from [crates.io](https://crates.io/crates/lyrical).

```sh
cargo install lyrical
```

Cargo will build the `lyrical` binary and place it in your `CARGO_INSTALL_ROOT`.
For more details on installation location see [the cargo book](https://doc.rust-lang.org/cargo/commands/cargo-install.html#description)

## Cargo (git)

Install this program from git by running this command.

```sh
cargo install --git https://github.com/tblelrd/lyrical
```

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
