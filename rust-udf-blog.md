# Extending MariaDB with UDFs using Rust

One of the most straightforward ways to add functionality to MariaDB or MySQL
server instances is by creating a user-defined function (UDF). These are
compiled functions loaded from dynamic binaries that can be much more performant
and flexible than stored functions or procedures written in SQL, providing the
same capabilities as builtin functions.

These UDFs are typically written in C or C++, but a library is now available
that makes it easy to write them in Rust. This blog discusses some of the
reasoning for writing this library, followed by a very basic usage example
that doesn't require any experience with the language.


## Why Rust?

Extensions to MariaDB can be written in anything that can produce a compiled
dynamic library, which is typically C or C++ (the same languages the server
itself is written in). There is nothing wrong with this current approach, but
being able to write them in Rust has some advantages:

* Protection from the most common relevant [CWE]s is guaranteed at compile time
  (specifically overread/overwrite, use after free, null dereference, and
  race conditions)
* Type safety can be leveraged to enforce code correctness
* Smart pointers prevent memory leaks (similar to C++, but Rust's implementation
  is somewhat more straightforward)
* Incredible toolchain; `cargo` is Rust's default build and dependency
  management system, which ships with every release. Out of the box, you get:
  * Compiling (`cargo check` / `cargo build`)
  * Linting (`cargo clippy`)
  * Testing (`cargo test`, alias `cargo t`)
  * Documentation (`cargo doc`, usually `cargo doc --document-private-items
    --open` for libraries)
  * Dependency management (configured in `Cargo.toml`)

Databases are applications where performance bottlenecks can easily bog down the
web services they drive. They must also take extreme care to avoid security
issues, since easy to miss things like buffer overreads can mean compromising
sensitive data. Rust is particularly well suited to this niche, providing
performance similar to (or even better than) C and C++, while guaranteeing
against those languages' most common security pitfalls.

If you aren't familiar with the language, you likely have questions like "how
can these guarantees be made while still allowing lower-level code and things like
C interfaces?" The answer is fairly straightforward; things that require
potentially unsound tasks (pointer operations, potentially thread-unsafe things,
inline assembly, C FFI) are possible within an `unsafe {...}` block (and only
within these blocks). This means that these small few lines of code can be
easily identified and thoroughly verified then wrapped within a safe API, and
anything built on top is guaranteed to be sound as long as the `unsafe` sections
are. Since the `udf` library handles all these `unsafe` operations for you, it
is possible to write almost any UDF entirely in safe code.

[CWE]: https://cwe.mitre.org/top25/archive/2022/2022_cwe_top25.html


## Example UDF Walkthrough

In this section, we will implement an extremely simple user-defined function
and cover its writing, building, and usage aspects.

If you would like to follow along, you will a Rust compiler with version ≥1.65
(because of dependence on GATs). If you don't yet have Rust, get it from
<https://rustup.rs/>. If you have it installed, run `rustup update` to ensure
you are on the latest version. If you are using an IDE, get the [rust-analyzer]
language server to help.

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

The above creates a directory called `test-udf` with `Cargo.toml` and `src/lib.rs`,
then verifies everything is working correctly (the `cargo new` command makes a
super simple example function and test). We need to update`Cargo.toml` to tell Cargo
to produce the correct kind of output (a dynamic library) and to use `udf`
as a depedency:

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
udf = "0.5" # our dependency on the `udf` crate
```

You can delete everything in `lib.rs` and our setup is complete.


### UDF Architecture

Let's write a very simple UDF that performs a running total of integers.

A UDF typically needs to make three symbols available to the server:

* An `init` call that validates argument type and performs memory allocation
* A `process` call run once per row that produces a result
* A `deinit` call that frees any memory from the setup

Common interfaces like this in Rust are grouped into `trait`s. The `BasicUdf`
trait is of interest here and provides interfaces for `init` and `process`
(`deinit` is handled automatically).

This trait should be implemented on a structure representing data to be shared
among calls to `process`, once per line. In this case, the data is just our
current total.

```rust,skt-default
struct RunningTotal(i64);
```

I am using an "tuple struct" syntax here which means you can access fields with
numbers (`some_struct.0`, `some_struct.1`) rather than by names
(`some_struct.field`). This is just a convenience as we only have one field, but
you are more than welcome to use a standard struct (they're identical behind the
scenes):

```rust,skt-default
struct RunningTotal {
  total: i64
}
```

We now need to do three things

* Import needed types and functions. `udf` has a `prelude` module with the most
  commonly needed imports, so we can just import everything there
* Implement the `BasicUdf` trait for our struct
* Add the `#[register]` macro to create the correct symbols

The minimum compiling code looks like this

```rust,skt-default
use udf::prelude::*;

struct RunningTotal(i64);

#[register]
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
fix inside the brackets (`ctrl`+`.` on VSCode), it will offer to autofill the
function signatures for you.)_

Woah function signatures! [The docs on `BasicUdf`] go into detail about what
everything here does, but let's break it down simply:

```rust,skt-default
type Returns<'a> = i64;
```

This is just where we specify the return type of our UDF. See [the docs] for
more information about possible return types. Here, since we will return a non-null
integer, we will return an `i64`. (Ignore the `<'a>` - that is only relevant
when returning references, which isn't applicable here).

```rust,skt-impl
fn init(cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {
    todo!()
}
```

This is our initialization function, which takes a configuration object `cfg`
and a list of arguments `args`. `Init` on these types is just a marker to
indicate where they're being used (it's tied to what methods are available).

The return type here is something called a [`Result`] which is a builtin `enum`.
Rust `enums` can be used as in C (to represent fixed values), but they are also
"tagged unions" (sometimes called "sum types"). This is a super helpful concept
where an instance of an enum can safely represent data that may be one type `or`
another, like a C `union` but with the interface to correctly figure out the type.
`Result`is used to indicate possible failure and has two variants, in this case
`Ok(Self)` or `Err(String)`. So, from our function signature we can tell that
the type of a successful function call will be `Self` (i.e., `RunningTotal` which
gets saved for later use) and an error will be a `String` (which gets displayed
to the user). Makes sense, right?

`todo!()` is a macro (anything ending with a `!` is) that is built in and just
matches whatever type signature is needed to compile. If we actually tried to
run it would fail, but we can at least verify that there are no errors in our
code structure:

```bash
$ cargo c
warning: `test-udf` (lib) generated 5 warnings (run `cargo fix --lib -p test-udf` to apply 5 suggestions)
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
```

There are warnings about unused arguments, but the basic structure is all set!

[The docs on `BasicUdf`]: https://docs.rs/udf/latest/udf/trait.BasicUdf.html
[the docs]: (https://docs.rs/udf/latest/udf/trait.BasicUdf.html#associatedtype.Returns)
[`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html


### UDF Implementation: `init`

Now that we have our basic structure, let's take a look at how to get some results.

The main goal of our `init` function is to validate arguments. Let's look at the
implementation then break it down:

```rust,skt-impl
fn init(_cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {
    if args.len() != 1 {
        return Err(format!("Expected 1 argument; got {}", args.len()));
    }

    // Coerce everything to an integer
    args.get(0).unwrap().set_type_coercion(SqlType::Int);

    Ok(Self(0))
}
```

The first part here checks out argument count:

```rust,skt-args-check
if args.len() != 1 {
    return Err(format!("Expected 1 argument; got {}", args.len()));
}
```

The number of arguments should be one. If not, it creates a formatted error
message string and returns it (`Err(something)` is how to construct a
`Result` enum error variant).

The second logical block:

```rust,skt-args-check
args.get(0).unwrap().set_type_coercion(SqlType::Int);
```

Uses `.get(0)` to attempt to get the first argument. This returns an
`Option<SqlArg>`which is another builtin enum type like `Result`.

`Option<T>` has two possible variants: `Some(T)` to represent an existing value
of type T, and `None` to represent nothing. `unwrap()` is used to get the inner
value out of a `Some()` value, or panic if there is `None`, so we use it to get
the argument at index 0 (first argument).

It should be noted that panicking is a very bad idea in UDFs and should absolutely
be avoided. We have already verified that there is a single argument here though,
so unwrapping is OK in this case.

```rust,skt-impl-ret
Ok(Self(0))
```

The last section simply creates a `Self` instance with 0 as its inner value, and
returns it as a successful call. That's all that is needed in the `init` phase.


### UDF Implementation: `process`

The `process` function is also fairly simple:

```rust,skt-impl
fn process<'a>(
    &'a mut self,
    _cfg: &UdfCfg<Process>,
    args: &ArgList<Process>,
    _error: Option<NonZeroU8>,
) -> Result<i64, ProcessError> {
    // Get the value as an integer and add it to our total
    self.0 += args.get(0).unwrap().value().as_int().unwrap_or(0);

    // The result is just our running total
    Ok(self.0)
}
```

The first line contains most of the logic, and uses combinators to keep things
terse. It does the following:

* `args.get(0).unwrap()`: this gets the first argument, as discussed above. We
  are OK to unwrap here because we validated our arguments (`init` and `process`
  get the same number of arguments when they are called, and we performed
  validation in init)
* `.value()` gets the argument's value, which is a `SqlResult`. This is an enum
  with variants for `String`, `Real`, `Int`, and `Decimal`.
* `.as_int()` is a convenience function that returns an `Option`. If the value
  is an nonnull integer, it will return `Some(i64)`. Any other possibilities
  will return None. Because we set type coercion in `init`, we can reasonably
  expect that all values will be an integer or null.
* `unwrap_or(0)` acts the same as `unwrap()` when the value is `Some`, but uses
  the specified value (0) when the value is `None`. This means that any null
  values have no effect on our sum.

The last line `Ok(self.0)` just returns our struct's inner value, which
represents our current running total. With that, our process function is
complete!


### Unit Testing

The `udf` crate provides functionality to thoroughly test UDF implementations
without even loading them into SQL. If you aren't familiar, it is worth looking
at the basics of Rust's [unit testing] for a brief outline.

We need to update our `udf` dependency to use the `mock` feature so we can
access that feature-gated module. Change the dependency line to look like the
following:

```toml
udf = { version = "0.5", features = ["mock"] }
```

And add the following below our test UDF:

```rust,skt-mocks
#[cfg(test)]
mod tests {
    // Use our parent module
    use super::*;
    use udf::mock::*;

    #[test]
    fn test_basic() {
        // Create a mock `UdfCfg` and mock `UdfArgs`
        let mut cfg = MockUdfCfg::new();
        let mut row_args = [
            // Each entry here acts as a row. Format for the macro is
            // `([type] value, "attribute name", nullable)`
            mock_args![(Int None, "", false)],
            mock_args![(10, "", false)],
            mock_args![(Int None, "", false)],
            mock_args![(-20, "", false)],
        ];

        // Run the `init` function on our mock data
        let mut rt = RunningTotal::init(cfg.as_init(), row_args[0].as_init()).unwrap();

        // Expected output after each of our "row"s
        let outputs = [0i64, 10, 10, -10];

        for (arglist, outval) in row_args.iter_mut().zip(outputs.iter()) {
            // Run the process function and verify the result
            let res = rt.process(cfg.as_process(), arglist.as_process(), None);
            assert_eq!(res, Ok(*outval));
        }
    }
}
```

Let's check the result:

```
running 1 test
test tests::test_basic ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Excellent!

See [this blog post's repository] for some more example unit tests. With good
unit testing, it is possible to be highly confident that a UDF performs as
expected without even loading it into our server.

[unit testing]: https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#unit-testing


### Loading the Function

The final test for our UDF is to actually run it in a server. Building a `.so`
file that can be loaded into MariaDB is easy (`cargo build --release`, output
is in `target/release`) but testing is easier with a dockerfile:

```Dockerfile
FROM rust:latest AS build

WORKDIR /build

COPY . .

# Use Docker buildkit to cache our build directory for quicker recompilations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release \
    && mkdir /output \
    && cp target/release/libMY_CRATE_NAME.so /output

FROM mariadb:10.10

COPY --from=build /output/* /usr/lib/mysql/plugin/
```

(this docker image uses cache which is a feature of Docker buildkit. Make sure
you are using a newer version of Docker, or have the environment variable set
correctly to enable it. Or, remove the cache indicators).

Be sure to update file name at `MY_CRATE_NAME` (`libtest_udf.so` if you
followed the earlier suggestion). The following runs and builds our image:

```bash
# Build the image
docker build . --tag mdb-blog-udf

# Run the image and name it mariadb_blog_udf for convenience
docker run --rm -d -e MARIADB_ROOT_PASSWORD=example --name mariadb_blog_udf mdb-blog-udf

# Enter the SQL console
docker exec -it mariadb_blog_udf mysql -pexample
```

Let's load our function and test it:

```sql
MariaDB [(none)]> CREATE FUNCTION running_total RETURNS integer SONAME 'libtest_udf.so';
Query OK, 0 rows affected (0.003 sec)

MariaDB [(none)]> select running_total(1, 2, 3);
ERROR 1123 (HY000): Can't initialize function 'running_total'; Expected 1 argument; got 3

MariaDB [(none)]> select running_total(10);
+-------------------+
| running_total(10) |
+-------------------+
|                10 |
+-------------------+
1 row in set (0.000 sec)
```

So far so good! Now a slightly harder test

```sql
MariaDB [(none)]> create database db; use db; create table t1 (val int);
Query OK, 1 row affected (0.000 sec)

Database changed
Query OK, 0 rows affected (0.023 sec)

MariaDB [db]> insert into t1(val) values (1),(2),(3),(NULL),(-100),(50),(123456789);
Query OK, 7 rows affected (0.002 sec)
Records: 7  Duplicates: 0  Warnings: 0

MariaDB [db]> select val, running_total(val) from t1;
+-----------+--------------------+
| val       | running_total(val) |
+-----------+--------------------+
|         1 |                  1 |
|         2 |                  3 |
|         3 |                  6 |
|      NULL |                  6 |
|      -100 |                -94 |
|        50 |                -44 |
| 123456789 |          123456745 |
+-----------+--------------------+
7 rows in set (0.000 sec)
```

Perfect!

## Wrapup

We have successfully written a simple UDF that validates its arguments and
stores some data between calls with just a few lines of code. We also performed
both unit and (non-automated) integration testing, which are easy steps to make
sure our program works as expected.

As mentioned, the code for this example is in [this blog post's repository]. For
anyone looking to explore a little further, the code could easily be turned into
an aggregate UDF by implementing the `AggregateUdf` trait.

Helpful links:

- `udf` library [repository page](https://github.com/pluots/sql-udf) and
  [documentation](https://docs.rs/udf/latest/udf/)
- MariaDB [UDF usage documentation](https://mariadb.com/kb/en/create-function-udf/)

[this blog post's repository]: https://github.com/tgross35/mariadb-udf-blog
