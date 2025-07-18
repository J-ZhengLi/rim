# Introduction

This is the usage documentation of this program (RIM),
aiming to help you master all the functionalities that RIM provides.

Although we tend to cover every bit of detail to make RIM easy to use, it is possible that
some documentation turns out to be unclear or missing entirely.
If that happens, please don't hesitate to notify us by open a new issue [here](https://gitcode.com/xuanwu/custom-rust-dist/issues),
or better, contribute to this project by improve our documentation and open a pull-request.

Anyway, let's start by introducing what this program is, how does it work, and why it's needed.

## What is RIM?

RIM, stands for **Rust Installation Manager**.
It is a program that allows you to easily manage your Rust's installation
using graphical user interface (GUI) or interactive commandline interface (CLI).

With RIM, you can:

1. Install Rust toolchain with `rustup` (we'll get to that later).
2. Install third-party tools with dependency control.
3. Update your Rust installation.
4. Modify your Rust installation such as add or remove components.
5. Check your Rust code and export report in different format. (WIP)
6. Export project template. (WIP)

(More to come...)

## How does it Work?

RIM come in two versions, the GUI version (the one you are using now),
and a light-weight CLI version that are normally used in operating systems without desktop environment (or docker container).

This GUI version also supports commandline interface, as long as you pass a `--no-gui` flag when executing its binary,
if you need more information, try starting this program with `--no-gui --help` on terminal.

### GUI

The GUI of RIM is written with [`Tauri`](https://v2.tauri.app/) framework,
and it is written using [`VueJS`](https://vuejs.org/) with [`TypeScript`](https://www.typescriptlang.org/).

### Toolkit

We all know that Rust itself offers a set of tools to distribute, such as `rustc`, `cargo`, `rust-std`, etc.
But different from Rust's toolchain, RIM installs Rust by installing a **toolkit**,
which is basically a Rust toolchain plus a set of extended tools that are not provided by official Rust,
such as binaries, IDE, plugins, or even crates, etc.

RIM relies on [`rustup`](https://github.com/rust-lang/rustup) (Rust's official toolchain manager) to install Rust toolchain,
so you don't have to worry that it might breaks your habits of how you use Rust.

RIM provides a more flexible way to install third-party tools instead of relying on a single package structure
like `rustup` does. Below is an example of how some packages are handled:

1. Binaries
   
    Place all binary files in the archive directly to `<INSTALL_ROOT>/cargo/bin/`.
2. Crate (binary)
   
    Compile from source code and place the artifacts into `<INSTALL_ROOT>/cargo/bin/` by calling `cargo install`.
3. Crate (dependency)
   
    Store the crate's source code in the `<INSTALL_ROOT>/crates/`, and modify `<INSTALL_ROOT>/cargo/config.toml` to use
    it as patch.
    > Note: There's a very noticeable side-effect, which triggers a warning when compiling a Rust project that does
    > not depends on such crate. We are still looking for better solutions.

They are all controlled by a single configuration file, also known as a **toolkit manifest**.

### Toolkit Manifest

A toolkit manifest controls all the information about a toolkit, such as its name, version, and:

1. Rust toolchain and where to get it.
2. What tools can be installed and where to get them.

Which means you can literally make your own Rust distribution by providing the package sources,
and distributes your toolkit manifest to the users!

## Why it's Needed?

When installing Rust using the traditional way, you need some preparations or configurations, that includes
setting the `CARGO_HOME` and `RUSTUP_HOME` environment variables if you don't want to install Rust in your `C:/` drive,
and setting `RUSTUP_DIST_SERVER`, `RUSTUP_UPDATE_ROOT` if your live in a country that the official
server is slow as snail, RIM does all those for you.

When you are using Rust, you might want some third-party tool, such as prerequisites (VS Build Tools or MinGW),
RIM can handle those for you.

Normally, this wouldn't be a trouble if you are an experienced Rust developer,
but for the countless of people that just got in the Rust ecosystem, RIM is a nice
tool to help them to start hacking in Rust quickly.
