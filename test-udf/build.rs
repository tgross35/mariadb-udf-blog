extern crate skeptic;

fn main() {
    // generates doc tests for `README.md`.
    skeptic::generate_doc_tests(&["../rust-udf-blog.md"]);
}
