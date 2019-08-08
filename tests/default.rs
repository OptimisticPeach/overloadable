#![feature(unboxed_closures, fn_traits)]
use std::fmt::Debug;
overloadable::overloadable! {
    pub(crate) func_name as
    fn(_: usize) {},
    fn<T>((x, y): (T, usize)) -> String where T: Debug {
        format!("{:?}, {:?}", x, y)
    },
    #[no_mangle]
    fn<'a, 'b: 'a>(a: &mut &'a str, b: &'b str) {
        *a = &b[..]
    }
}

#[test]
fn it_works() {
    assert_eq!(func_name((1, 2)), "1, 2");
    let a = "abc";
    {
        let mut b = "def";
        func_name(&mut b, a);
        assert_eq!(b, "abc");
    }
}

pub struct Foo1;

overloadable::overloadable_member! {
    pub Foo1::func_name as
    fn(_: usize) {},
    fn<T>((x, y): (T, usize)) -> String where T: Debug {
        format!("{:?}, {:?}", x, y)
    },
    #[no_mangle]
    fn<'a, 'b: 'a>(self, a: &mut &'a str, b: &'a str) {
        *a = &b[..]
    },
    fn(&self) -> usize {1}
}

//Forum example:
#[derive(Clone)]
enum Foo {
    A,
    B,
}
overloadable::overloadable_member! {
    Foo::my_func as
    fn(&self) -> &'static str {
        match self {
           Foo::A => "A",
           Foo::B => "B",
        }
    },
    fn(self, x: usize) -> Vec<Self> {
        let mut val = Vec::new();
        val.resize_with(x, || self.clone());
        val
    },
    fn() -> Box<Self> {
        Box::new(Foo::A)
    },
    fn(self: Box<Self>, other: Box<Self>) -> usize {
        match (&*self, &*other) {
            (Foo::A, Foo::A) => 2,
            (Foo::B, Foo::A) | (Foo::A, Foo::B) => 1,
            _ => 0
        }
    }
}
