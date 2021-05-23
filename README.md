# opus-native

[![Latest version](https://img.shields.io/crates/v/opus-native.svg)](https://crates.io/crates/opus-native)
[![Documentation](https://docs.rs/opus-native/badge.svg)](https://docs.rs/opus-native)
![BSD-3](https://img.shields.io/badge/license-BSD3-blue.svg)

## Overview

Implements the free and open audio codec Opus in Rust.

## Status

This crate is under heavy development. Most functionality is not working.

### TODO

* Decoder
* Encoder
* SIMD optimization
* Repacketizer
* Multistream decoder
* Multistream encoder

## Not supported Opus features

To decrease the implementation complexity, we don't support the following rarely used features:

* Fixed point arithmetic - Only float point arithmetic is supported
  (only affects cheap MCU without proper floating point support).
* Custom modes - The Opus specification allows the usage of custom modes (custom sample rates or
  custom frame sizes) as an optional feature. Since this breaks the compatibility with other
  programs, leads to worse encoding quality and is extremely rarely used, we decided to not support
  this optional feature.

If you need these Opus features, you must stick to the reference implementation.

## Target feature optimization

This crate enables SIMD intrinsics for the x86 and x86_64 targets on the stable Rust compiler for
SSE automatically.

AVX optimizations are available. You need to compile with the `avx` target feature to enable these.

If you wish to use SIMD intrinsics on non x86 and x86_64 platforms, you need to use the `nightly`
feature described below.

## Crate features

* `nightly` - Enables target specific SIMD intrinsics that are only currently available on the
  nightly Rust compiler. Affected target features: `neon` for `arm` / `aarch64`

## Credits

This crate is a direct port of the Opus reference implementation written in C:

[Reference implementation](https://gitlab.xiph.org/xiph/opus)

It implements the following specifications:

* [rfc6716](https://tools.ietf.org/html/rfc6716.html)
* [rfc7845](https://tools.ietf.org/html/rfc7845.html)
* [rfc8251](https://tools.ietf.org/html/rfc8251.html)

## License

Licensed under BSD-3-Clause.
