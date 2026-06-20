# fixedmod

[![Crates.io](https://img.shields.io/crates/v/fixedmod.svg)](https://crates.io/crates/fixedmod)
[![Documentation](https://docs.rs/fixedmod/badge.svg)](https://docs.rs/fixedmod)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.78.0%2B-blue.svg?maxAge=3600)](https://github.com/rust-lang/fixedmod)
![no_std](https://img.shields.io/badge/no__std-supported-success)
![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success)

Fast modular arithmetic for a fixed modulus — no division on the hot path.

```rust
use core::num::NonZeroU32;
use fixedmod::Modulus;

let m = Modulus::new(NonZeroU32::new(7).unwrap());

assert_eq!(m.mul_mod(5, 6), 2);
assert_eq!(m.pow_mod(3, 4), 81 % 7);
assert_eq!(m.reduce32(10), 3);
assert!(m.is_divisible(14));
assert_eq!(m.inv(3), Ok(5));
```

Integer division is slow.
`Modulus::new` precomputes everything needed once, so almost every operation runs without division.

## API

| Method               | Description                                 | No division |
| -------------------- | ------------------------------------------- | ----------- |
| `new(m)`             | precomputes magic numbers                   | N           |
| `mul_mod(a, b)`      | `a * b % m`                                 | Y           |
| `pow_mod(a, exp)`    | `a.pow(exp) % m`                            | Y           |
| `reduce32(a)`        | `a % m` for `u32`                           | Y           |
| `reduce64(a)`        | `a % m` for `u64`                           | Y           |
| `reduce64_signed(a)` | `a.rem_euclid(m as i64)`                    | Y           |
| `is_divisible(a)`    | `a % m == 0`                                | Y           |
| `inv(a)`             | modular inverse of `a`, or `Err(gcd(a, m))` | N           |

## License

MIT OR Apache-2.0
