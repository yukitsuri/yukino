# Third-Party Libraries and the Yukino Ecosystem

This document serves as the directory for official and community libraries for the Yukino programming language. And the rules for adding your library are just like in any civilized internet community.

---

## 1. About the Package Directory

Yukino is an experimental project created by a student developer. For this reason, dedicated servers, centralized package registries, and cloud infrastructure (similar to crates.io or npm) are currently not hosted (and if you have nothing better to do and want to fix this, feel free to reach out on the Discord server).

The `yukino` package manager was intentionally designed to be decentralized: it directly resolves dependencies from Git repositories (`git = "..."`), direct archive downloads (`link = "..."`), and local filesystem paths (`path = "..."`).

If you have written a useful library for Yukino and wish to share it with the community, you can submit it to this directory. Simply open a **Pull Request** adding your library's description, repository URL, and usage example.

---

## 2. Official Libraries

This section lists libraries maintained and developed by the team at [yukitsuri](https://github.com/yukitsuri). If you trust the Yukino compiler and runtime, these libraries are likely safe to trust as well.

*Catalog:*

[shareui/buildtool](https://github.com/shareui/buildtool): Compiles C/C++/Rust in a Yukino project.

---

## 3. Community Libraries

This section showcases packages, tools, bindings, and utilities contributed by independent developers and the Yukino community.

*Catalog:*

*Empty yet*

---

[← Previous: Standard Library Overview](en/stdlib/0-overview.md) | [Back to Index: Documentation Overview →](en/0-overview.md)
