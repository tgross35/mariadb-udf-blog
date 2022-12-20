#[cfg(not(miri))]
include!(concat!(env!("OUT_DIR"), "/skeptic-tests.rs"));
