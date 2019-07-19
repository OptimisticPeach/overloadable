# overloadable

Easy to use overloadable functions in rust using nightly features.

This crate provides you with the capabilities to overload your functions in a similar style to C# or C++, including support for meta attributes, type parameters and constraints, and visibility modifiers. 
Please visit the documentation for futher information. 

# Note
This is a **nightly** crate. You _must_ include the following line in your code for this crate to compile:
```rust
#![feature(unboxed_closures, fn_traits)]
```

## Example:

```rust
#![feature(unboxed_closures, fn_traits)]
use overloadable::overload;

overload! {
    pub func as
    #[inline(always)]
    fn(x: usize, y: usize) -> usize {
        x * y
    }
    fn<'a>(x: &'a usize) -> f32 {
        *x as f32
    }
    fn<'a, T>(x: &'a [T]) -> &'a T where T: std::fmt::Debug {
        println!("Found {:?}", &x[0]);
        &x[0]
    }
}

fn foo {
    assert_eq!(func(2, 3), 6);
    assert_eq!(func(&32), 32.0);
    assert_eq!(func(&[1, 2, 3, 4] as &[usize]), &0);
}
```