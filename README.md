# fixedmod

Fast modular arithmetic for a fixed modulus — no division on the hot path.

```rust
use core::num::NonZeroU32;
use fixedmod::Modulus;

let m = Modulus::new(NonZeroU32::new(7).unwrap());

assert_eq!(m.mul_mod(5, 6), 2);
assert_eq!(m.pow_mod(3, 4), 81 % 7);
assert_eq!(m.reduce32(10), 3);
assert!(m.divisible(14));
assert_eq!(m.inv(3), Ok(5));
```

Integer division is slow.
`Modulus::new` precomputes everything needed once, so almost every operation runs without division.

## API

| Method            | Description                                 | No division |
| ----------------- | ------------------------------------------- | ----------- |
| `new(m)`          | precomputes magic numbers                   | N           |
| `mul_mod(a, b)`   | `a * b % m`                                 | Y           |
| `pow_mod(a, exp)` | `a.pow(exp) % m`                            | Y           |
| `reduce32(a)`     | `a % m` for `u32`                           | Y           |
| `reduce64(a)`     | `a % m` for `u64`                           | Y           |
| `divisible(a)`    | `a % m == 0`                                | Y           |
| `inv(a)`          | modular inverse of `a`, or `Err(gcd(a, m))` | N           |

`no_std` compatible.

## License

MIT OR Apache-2.0
