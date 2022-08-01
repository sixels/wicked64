pub trait Callable<const NARGS: usize, A: Sized, O> {
    fn addr(&self) -> usize;

    fn nargs(&self) -> usize {
        NARGS
    }
}

impl<O> Callable<0, (), O> for fn() -> O {
    fn addr(&self) -> usize {
        *self as usize
    }
}
impl<A, O> Callable<1, A, O> for fn(A) -> O {
    fn addr(&self) -> usize {
        *self as usize
    }
}
impl<A, B, O> Callable<2, (A, B), O> for fn(A, B) -> O {
    fn addr(&self) -> usize {
        *self as usize
    }
}
impl<A, B, C, O> Callable<3, (A, B, C), O> for fn(A, B, C) -> O {
    fn addr(&self) -> usize {
        *self as usize
    }
}
impl<A, B, C, D, O> Callable<4, (A, B, C, D), O> for fn(A, B, C, D) -> O {
    fn addr(&self) -> usize {
        *self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_should_get_the_right_callable_arguments_size() {
        fn a() {}
        fn b(_: u8) {
            unimplemented!()
        }
        fn c(_: u8, _: u16) -> u8 {
            unimplemented!()
        }
        fn d(_: u8, _: u16, _: u32) -> String {
            unimplemented!()
        }
        fn e(_: u8, _: u16, _: u32, _: Vec<u64>) -> &'static str {
            unimplemented!()
        }

        assert_eq!(0, Callable::nargs(&(a as fn())));
        assert_eq!(1, Callable::nargs(&(b as fn(_) -> _)));
        assert_eq!(2, Callable::nargs(&(c as fn(_, _) -> _)));
        assert_eq!(3, Callable::nargs(&(d as fn(_, _, _) -> _)));
        assert_eq!(4, Callable::nargs(&(e as fn(_, _, _, _) -> _)));
    }
}
