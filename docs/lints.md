# Lints

We employ comprehensive and strict linting.

## Rust Compiler Lints

Find all the lints in
https://doc.rust-lang.org/beta/rustc/lints/listing/allowed-by-default.html. All
of those lints should be found in `[lints.rust]` in `Cargo.toml`, and the ones
we want to allow (which is the minority) should be commented out. Note that
`warnings = "deny"` is included, so we don't need to worry about any of the
warnings-by-default lints.

Each lint that is allowed should be listed in a commented-out line with an
explanation comment.

## Clippy Lints

Clippy lints are handled in the `Cargo.toml` file as well, under the
`[lints.clippy]` header. Our approach here is to simply list all lint
categories at the top of the section as denied:

```
cargo = { level = "deny", priority = -1 }
complexity = { level = "deny", priority = -1 }
correctness = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
perf = { level = "deny", priority = -1 }
restriction = { level = "deny", priority = -1 }
style = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
```

And then add the individual lints we want to alow below, with explanations.

Every now and then, make sure there are no new categories that need to be added
to the list by checking
https://rust-lang.github.io/rust-clippy/stable/index.html under the filter menu
"Lint groups". We never add the "deprecated" category.
