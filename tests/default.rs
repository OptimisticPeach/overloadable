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
