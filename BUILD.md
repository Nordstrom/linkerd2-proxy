# Linkerd2-Proxy Development Guide

This document will help you build and run Linkerd2 from source.

## Cargo

Usually, [Cargo][cargo], Rust's package manager, is used to build and test this
project. If you don't have Cargo installed, we suggest getting it via
https://rustup.rs/.

## Building the project

A `Makefile` is provided to automate most build tasks. It provides the
following targets:

* `make build` -- Compiles the proxy on your local system using `cargo`
* `make clean` -- Cleans the build target on the local system using `cargo clean`
* `make test` -- Runs unit and integration tests on your local system using `cargo`
* `make test-flakey` -- Runs _all_ tests, including those that may fail spuriously
* `make package` -- Builds a tarball at
  `target/release/linkerd2-proxy-${PACKAGE_VERSION}.tar.gz`. If
  `PACKAGE_VERSION` is not set in the environment, the local git SHA is used.
* `make docker` -- Builds a Docker container image that can be used for testing.
   If the `DOCKER_TAG` environment variable is set, the image is given this
   name. Otherwise, the image is not named.

*Note:* If you plan on using a debugger then you will want to turn on debug symbols in the [Cargo.toml](Cargo.toml) as follows:
```
[profile.dev]
debug = true 
```

## Running Locally for Development

You can run the proxy locally, outside of a k8s environment for development. 

```
LINKERD2_PROXY_LOG=debug LINKERD2_PROXY_POD_NAMESPACE=linkerd cargo run
```

You should then be able to proxy a request.

```
docker run -p 8080:80 nginx:1.7.9
curl -iv 127.0.0.1:4140 -H "Host: localhost:8080"
```
