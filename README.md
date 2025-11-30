# Watsup 🕰️

[![Build Status](https://github.com/fechu/watsup/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/fechu/watsup/actions/workflows/rust.yml)

> **A blazingly fast time tracker written in Rust 🦀 inspired by [Watson](https://github.com/jazzband/watson/)** 

Watsup is here to help you manage your time with **speed** and **efficiency**! Want to know how much time you're spending on your projects? Need to generate a report for your client? Watsup has got you covered.

Inspired by the wonderful [Watson](https://github.com/jazzband/watson/), Watsup brings the same intuitive CLI experience you love, but supercharged with Rust's performance and reliability. Best of all? **Watsup is fully compatible with Watson's data storage** – you can use both tools interchangeably without missing a beat! 🎉

> [!WARNING] 
> Watsup is still in early development and is not as complete as [watson](https://github.com/jazzband/watson/) and may contain bugs. Please report any issues you encounter on the [GitHub repository](https://github.com/fechu/watsup).

## Why Watsup?

- **⚡ Blazingly Fast**: Built with Rust for maximum performance
- **🔄 Watson Compatible**: Uses the same data format as Watson – switch between them seamlessly!
- **🎯 Familiar API**: If you know Watson, you already know Watsup
- **📦 Zero Dependencies**: Single binary, no runtime required

## Quick Start

### Installation

#### From Source

```bash
git clone https://github.com/fechu/watsup.git
cd watsup
cargo build --release
```

The binary will be available at `target/release/watsup`.

### Usage

Start tracking your activity:

```bash
$ watsup start world-domination
```

With this command, you've started a new **frame** for the _world-domination_ project . That's it!

Now stop tracking your world domination plan:

```bash
$ watsup stop
```

You can view your recent activity with the `log` command:

```bash
$ watsup log
Monday 15 January 2024 (0h 42min)
  world-domination  13:00 - 13:42  42m 15s
```

Check what you're currently working on:

```bash
$ watsup status
Project world-domination started 8 minutes ago
```

Cancel your current frame if you started tracking by mistake:

```bash
$ watsup cancel
Canceling the timer for project world-domination
```

List all your projects:

```bash
$ watsup projects
world-domination
...
```

Edit a frame (opens your `$EDITOR`):

```bash
$ watsup edit
```

## Contributing

Contributions are welcome! Whether it's:
- Bug reports
- Feature requests
- Documentation improvements
- Code contributions

Feel free to open an issue or submit a pull request!

## License

Watsup is released under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

A huge thanks to the [Watson](https://github.com/jazzband/watson/) team for creating such an intuitive time tracking tool and inspiring this project!

---

**Made with ❤️ and 🦀**
