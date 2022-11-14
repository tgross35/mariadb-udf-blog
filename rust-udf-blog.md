# Extending MariaDB with UDFs using Rust

The typical way of extending 

One of the most straightforward ways to extend MariaDB or MySQL server instances
without writing a function in SQ
is by creating a user-defined function (UDF).


The goal of the 

## Why Rust?

Typically extensions to MariaDB are written in C or C++ (since that is what the
database itself is written in).  However, this brings some limitations:


Databases are a foundation of internet connectivity. This places them at the intersection
of security and performance: any lag may noticibly reduce user experience, but security
issues like buffer overreads may mean compromising or leaking user data. This is a niche that Rust is 
_particularly_ well adapted to, and it provides multiple benefits over the C status quo:

### Security

Perhaps  
 
### "API is Documentation"



 
### Code _correctness_

in Rust it is possible to check typing 

### 

- Memory usage: Rust tends to do a good job of enforcing the 
  - Code cleans up



## Examle UDF Walkthrough

In this section, we will implement two extremely simple user-defined functions, and cover their
writing, building, and using aspects.


If you would like to follow along, you will need a copy of the Rust compiler >= 1.65. If
you don't yet have Rust, get it from <https://rustup.rs/>. If you have it installed, run
`rustup update` to get the latest version.

```bash
cargo new test-udf;
```

```toml

```

## Behind the scenes

