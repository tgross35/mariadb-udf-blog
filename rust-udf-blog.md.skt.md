# skeptic template file

This file just holds code blocks used by the main blog's `.md` file to fill in
the blanks so our doctests compile.

```rust,skt-default
use udf::prelude::*;

fn main() {{
    {}
}}
```

```rust,skt-impl
use udf::prelude::*;

struct Doctest(i64);

impl Doctest {{
    {}
}}

fn main() {{}}
```

```rust,skt-impl-ret
use udf::prelude::*;

struct Doctest(i64);

impl Doctest{{
    fn x() -> Result<Self,()> {{
        {}
    }}
}}

fn main() {{}}
```

```rust,skt-args-check
use udf::prelude::*;

fn doctest(args: &ArgList<Init>) -> Result<(), String> {{
    {}
    todo!()
}}

fn main() {{}}
```


```rust,skt-mocks
use udf::prelude::*;

#[derive(Debug, PartialEq)]
struct RunningTotal(i64);

#[register]
impl BasicUdf for RunningTotal {{
    type Returns<'a> = i64;

    fn init(_cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {{
        if args.len() != 1 {{
            return Err(format!("Expected 1 argument; got {{}}", args.len()));
        }}

        // Coerce everything to an integer
        args.get(0).unwrap().set_type_coercion(SqlType::Int);

        Ok(Self(0))
    }}

    fn process<'a>(
        &'a mut self,
        _cfg: &UdfCfg<Process>,
        args: &ArgList<Process>,
        _error: Option<NonZeroU8>,
    ) -> Result<Self::Returns<'a>, ProcessError> {{
        // Get the value as an integer and add it to our total
        self.0 += args.get(0).unwrap().value().as_int().unwrap_or(0);

        // The result is just our running total
        Ok(self.0)
    }}
}}

fn main() {{}}

{}
```
