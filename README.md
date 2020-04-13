# Unprolix

[![Crate](https://img.shields.io/crates/v/unprolix.svg)](https://crates.io/crates/unprolix)
[![Documentation](https://docs.rs/unprolix/badge.svg)](https://docs.rs/unprolix)
[![Travis Status](https://travis-ci.org/vlopes11/unprolix.svg?branch=master)](https://travis-ci.org/vlopes11/unprolix)

Remove boilerplate for constructors, getters and setters. Make your code even cleaner!

Unprolix benefits from procedural derive macros.

This version is experimental and was used for a few personal projects. No issues have been found so far, but you are welcome to report any problem.

# Examples

```rust
use unprolix::{Constructor, Getters, Setters};

mod the_mod {
    #[derive(Default)]
    pub struct TheModStruct {
        a: bool,
    }
}

#[derive(Constructor, Getters, Setters)]
struct SomeMultipleZ {
    flag: bool,
    pub x: usize,

    // default will make the constructor not expect this attribute as argument
    // copy will make the getter copy the value, instead of passing via reference
    #[unprolix(default, copy)]
    y: usize,

    z: (u8, u8),

    // skip will not generate getters and setters
    #[unprolix(skip)]
    s: u8,

    // as_slice will be called from a `Vec` instead of passing vec as ref
    #[unprolix(as_slice)]
    l: Vec<i32>,

    w: the_mod::TheModStruct,
}

fn main() {
    let flag = true;
    let x = 5;
    // y is defaulted
    let z = (1u8, 2u8);
    let s = 0u8;
    let l = vec![3, 5, 7];
    let w = the_mod::TheModStruct::default();

    let mut m = SomeMultipleZ::new(flag, x, z, s, l, w);

    assert_eq!(&true, m.flag());
    m.set_flag(false);
    assert_eq!(&false, m.flag());

    // x is not created because its public

    // y is returned as value because copy is present
    let _a: usize = m.y();

    // s doesnt implement getters or setters because of skip

    // l returns a slice instead of a reference to a vector
    let _a: &[i32] = m.l();
    m.l_as_mut().push(25);
}
```
