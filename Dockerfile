# Dockerfile to build the udf-examples crate and load it. Build with:
#
# ```
# docker build . --tag mdb-blog-udf
# ```
#
# If using this for production, be sure to remove the `MARIADB_ROOT_PASSWORD`
# directive.

FROM rust:latest AS build

WORKDIR /build

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --manifest-path test-udf/Cargo.toml  \
    && mkdir /output \
    && cp test-udf/target/release/libtest_udf.so /output

FROM mariadb:10.9

COPY --from=build /output/* /usr/lib/mysql/plugin/

# # Do NOT use this for production
ENV MARIADB_ROOT_PASSWORD=example
