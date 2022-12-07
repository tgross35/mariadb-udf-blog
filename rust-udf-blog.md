# Extending MariaDB with UDFs using Rust

One of the most straightforward ways to add functionality to MariaDB or MySQL
server instances is by creating a user-defined function (UDF). These are
compiled functions loaded from dynamic binaries that can be much more performant
and flexible than SQL-based options, providing similar functionality to builtin
functions.

These functions are typically written in C or C++, but, a new library is
available that makes it easy to write them in Rust. This blog discusses some of
the reasoning for writing this library, followed by a very basic example that
doesn't require any experience with the language.


## Why Rust?

Extensions to MariaDB can be written in anything that can produce a compiled
dynamic library, which is typically C or C++ (the same languages the server
itself is written in). There is absolutely nothing incorrect with this current
approach, but being able to write them in Rust has some advantages:

* Protection from the most common [CWE]s (with the focus being on
  overread/overwrite, use after free, null dereference)
* Type safety can be leveraged to enforce code _correctness_
* RAII prevents memory leaks. Rust's implementation is somewhat more
  straightforward than in C++
* API is documentation
* Incredible toolchain; `cargo` is Rust's default build and dependency
  management system, which ships with every release. Out of the box, you get:
  * Compiling (`cargo check` / `cargo build`)
  * Linting (`cargo clippy`)
  * Testing (`cargo test`, alias `cargo t`)
  * Documentation (`cargo doc`, usually `cargo doc --document-private-items
    --open`)
  * Dependency management (configured in `Cargo.toml`)

Databases, being a foundation of internet connectivity are at the intersection
of security and performance: any lag may noticibly reduce user experience, but
security issues like buffer overreads may mean compromising or leaking user
data. This is a niche that Rust is _particularly_ well adapted to.

If you aren't familiar with the language, you might be tempted to ask something
like "how can something like a C interface or performance-oriented...?" The
answer is fairly straightforward; things that require low-level tasks like
pointer operations are possible within an `unsafe {...}` block. The `udf`
library handles all these `unsafe` operations for you to provide a safe API.

[CWE]: https://cwe.mitre.org/top25/archive/2022/2022_cwe_top25.html


## Example UDF Walkthrough

In this section, we will implement two extremely simple user-defined functions,
and cover their writing, building, and using aspects.


If you would like to follow along, you will need a copy of the Rust compiler >=
1.65. If you don't yet have Rust, get it from <https://rustup.rs/>. If you have
it installed, run `rustup update` to ensure you are on the latest version. If
you are using an IDE, get the [rust-analyzer] language server to help.

[rust-analyzer]: https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer


### Workspace setup

The first step is to create a new project; Cargo makes this pretty easy;

```bash
# Create a new Rust project called test-udf
cargo new --lib test-udf

cd test-udf

# Simply validate that the code compiles, same as `cargo c`
cargo check

# Run the unit test, same as `cargo t`
cargo test
```

The above creates a directory called `test-udf` with a `Cargo.toml` file, and a
`src/lib.rs` file with a simple function and test for it. We need to update
`Cargo.toml` to tell Cargo to use `udf` as a depedency and to produce the
correct kind of output.


```toml
[package]
name = "test-udf"
version = "0.1.0"
edition = "2021"
publish = false # prevent accidentally publishing to crates.io

# Add this section to specify we want to create a C dynamic library
[lib]
crate-type = ["cdylib"]

[dependencies]
udf = "0.4" # our dependency on the `udf` crate
```

You can delete everything in `lib.rs` and our setup is complete.


### UDF Architecture

Let's write a super simple UDF that performs a running total of integers.

A UDF needs to provide three symbols to the server:

* An `init` call that validates areument type and performs memory allocation
* A `process` call run once per row that produces a result
* A `deinit` call that frees any memory from the setup

Common interfaces like this in Rust are grouped into `trait`s. The `BasicUdf`
trait is of interest here and provides interfaces for `init` and `process`
(`deinit` is handled automatically).

This trait should be implemented on a structure representing data to be shared
among calls to `process`, once per line. In this cast, the data is just our
current total.

```rust,skt-default
struct RunningTotal(i64);
```

I am using an "tuple struct" syntax here which just means you can access fields
with numbers (`some_struct.0`, `some_struct.1`) rather than by names
(`some_struct.field`). This is just a convenience as we only have one field, but
you are more than welcome to use a standard struct (they're identical behind the
scenes)

```rust,skt-default
struct RunningTotal {
  total: i64
}
```

We need to do three things

* Import needed types and functions. `udf` has a `prelude` module with the most
  commonly needed imports, so we can just import everything there
* Implement a trait for our struct

The minimum compiling code looks like this

```rust,skt-default
use udf::prelude::*;

struct RunningTotal(i64);

impl BasicUdf for  RunningTotal {
    type Returns<'a> = i64;

    fn init(cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {
        todo!()
    }

    fn process<'a>(
        &'a mut self,
        cfg: &UdfCfg<Process>,
        args: &ArgList<Process>,
        error: Option<NonZeroU8>,
    ) -> Result<Self::Returns<'a>, ProcessError> {
        todo!()
    }
}
```

_(hint: if you just type `impl BasicUdf for RunningTotal {}` then open the quick
fix inside the brackets (`ctrl`+`.` on VSCode), it will autofill the function
signatures for you.)_

Woah function signatures! [The docs on `BasicUdf`] go into detail about what
everything here does, but let's break it down simply:

```rust,skt-default
type Returns<'a> = i64;
```

This is just where we specify the return type of our UDF. See [the docs] for
more information about possible return types. Here, since we are working on
integers, we will return an `i64`.

```rust,skt-default
# struct X;
# impl X {
fn init(cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {
    todo!()
}
# }
```

This is our initialization function, which takes a configuration object `cfg`
and a list of arguments `args`. `Init` on these types is just a marker to
indicate where they're being used (it's tied to what methods are available).

The return type here is something called a [`Result`] which is a builtin `enum`.
Rust `enums` can be used as in C (to represent fixed values), but they are more
common as "tagged unions" (sometimes called "sum types"). This is where an
instance of an enum can represent data that can be more than one type. `Result`
is used to indicate possible failure and has two variants, in this case
`Ok(Self)` and `Err(String)` (represented by generic types).

`todo!()` is a macro (anything ending with a `!` is) that just matches whatever
type signature is needed to compile. If we actually tried to run it would fail,
but we can at least verify that there are no errors in our code structure:

```bash
$ cargo c
warning: `test-udf` (lib) generated 5 warnings (run `cargo fix --lib -p test-udf` to apply 5 suggestions)
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
```

There are warnings about unused arguments, but the Basic structure is all set!

[The docs on `BasicUdf`]: https://docs.rs/udf/latest/udf/trait.BasicUdf.html
[the docs]: (https://docs.rs/udf/latest/udf/trait.BasicUdf.html#associatedtype.Returns)
[`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html


### UDF Implementation

Now that we have our basic structure, let's take a look at how to get some results.

The main goal of our `init` function is to validate arguments.


### Unit Testing


### Loading the Function


### Integration Testing


### Aggregate UDF


## Behind the scenes
