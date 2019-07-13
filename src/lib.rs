/*!
  This crate provides the ability to create overloadable functions in rust through
  the use of a macro.

  # Syntax:

  ```no_run
  use overloadable::overloadable;
  overloadable!{
      #[my_meta]
      visibility function_name as
      fn<OptionalTypeArgs>(function_params: function_types) -> optional_return_type where [OptionalTypeArgs: constraints] {
          code_body
      }
  }
  ```

  # What is produced

  Here is an example of the output produced by `overloadable`:
  ```
  use overloadable::overloadable;
  use std::fmt::Debug;
  overloadable!{
      pub my_func as
      fn(x: usize, y: &str) -> f32 {
          (x * y.len()) as f32
      }
      fn<T>() where [T: Debug] {}
  }
  //Gives
  #[allow(non_camel_case_types)]
  pub struct my_func;
  impl Fn<(usize, &str,)> for my_func {
      extern "rust-call" fn call(&self, (x, y,): (usize, &str,)) -> f32 {
          {
              (x * y.len()) as f32
          }
      }
  }
  //The rest of the `Fn*` family
  impl<T> Fn<()> for my_func where T: Debug {
      extern "rust-call" fn call(&self, (): ()) -> () {
          {}
      }
  }
  //The rest of the `Fn*` family.
  ```

  # Limitations
  - These functions cannot be exposed for ffi.
  - These functions cannot be `const fn`s.
  - These functions cannot pattern match their arguments.
  - These functions cannot be used in place of a `fn()`-style function pointer
    - But they can be used with the `Fn*` family of traits.
  - The `where` clause' contents must be surrounded by square brackets due to a constraint in macros.
  - Generic lifetime parameters must always be proceeded by a comma, even if they are the only generic parameters.

*/

#![feature(unboxed_closures, fn_traits)]
#[macro_export]
macro_rules! overloadable {
    ($v:vis $name:ident as
    $(
        $(#[$($m:tt)*])*
        fn
            $(
                <$($lt:lifetime,)*
                 $($gen:ident),*$(,)?>
            )?
            (
                $($param:ident : $param_type:ty$(,)?)*
            )
            $( -> $ret:ty)?
            $(where [$($cons:tt)*])?
        $code:block
    )*) => {

        #[allow(non_camel_case_types)]
        $v struct $name;
        $(
            impl$(<$($lt,)*$($gen)?>)? ::std::ops::Fn<($($param_type,)*)> for $name where $($($cons)*)? {
                $(#[$($m)*])*
                extern "rust-call" fn call(&self, ($($param,)*): ($($param_type,)*)) -> <Self as FnOnce<($($param_type,)*)>>::Output {
                    $code
                }
            }

            impl$(<$($lt,)*$($gen)?>)? ::std::ops::FnMut<($($param_type,)*)> for $name where $($($cons)*)? {
                $(#[$($m)*])*
                extern "rust-call" fn call_mut(&mut self, x: ($($param_type,)*)) -> <Self as FnOnce<($($param_type,)*)>>::Output {
                    <Self as Fn<($($param_type,)*)>>::call(self, x)
                }
            }

            impl$(<$($lt,)*$($gen)?>)? ::std::ops::FnOnce<($($param_type,)*)> for $name where $($($cons)*)? {
                type Output = ($($ret)?);
                $(#[$($m)*])*
                extern "rust-call" fn call_once(self, x: ($($param_type,)*)) -> <Self as FnOnce<($($param_type,)*)>>::Output {
                    <Self as Fn<($($param_type,)*)>>::call(&self, x)
                }
            }
        )+
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    overloadable! {
        pub(self) func_name as
        #[inline(always)]
        fn(x: usize, y: &str) -> f32 {
            (x * y.len()) as f32
        }
        #[inline(never)]
        fn(z: usize, y: bool) -> isize {
            if y {
                z as _
            } else {
                z as isize * -1
            }
        }
        #[no_mangle]
        fn() {
            println!("Abc");
        }
        fn<'a,>(x: &'a usize, y: &'a usize, z: &'a usize) {}
        fn<T>(x: &T) -> String where [T: Debug] {
            format!("{:?}", x)
        }
        fn<'a, T>(x: &'a T, y: &'a T) where [T: Debug] {}
    }

    #[test]
    fn it_works() {
        assert_eq!(func_name(2, "abc"), 6.0f32);
        assert_eq!(func_name(3, false), -3);
        assert_eq!(func_name(), ());
        assert_eq!(func_name(&String::from("foo")), String::from(r#""foo""#));
    }
}
