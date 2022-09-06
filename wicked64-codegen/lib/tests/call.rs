use w64_codegen::emit;
use w64_codegen::Emitter;

#[test]
fn call_zero() {
    static mut STATUS: Option<String> = None;

    // let mut emitter = Emitter::new();
    let mut emitter = Emitter::default();

    fn foo() {
        unsafe {
            STATUS = Some(String::from("Success"));
        }
    }

    emit!(emitter,
        call_fn foo();
        ret;
    );

    // let code = emitter.finalize().unwrap();
    let code = unsafe { emitter.make_exec() }.unwrap();

    code.execute();

    assert_eq!(unsafe { STATUS.clone() }, Some(String::from("Success")));
}

#[test]
fn call_many() {
    struct Data<'s> {
        bx: Box<[u64]>,
        string: &'s str,
    }

    let (pa, pb): (u8, i32) = (0xfe, -0x21f0);
    fn primitives(a: u8, b: i32) {
        assert_eq!(a, 0xfe);
        assert_eq!(b, -0x21f0);
    }

    // `call_fn` requires all ref arguments to be `Sized`,
    // we don't need to care about pointer metadata.
    fn references(a: &String, b: &Vec<String>, c: &Data) {
        assert_eq!(a, "First Argument");
        assert_eq!(b, &[String::from("Second"), String::from("Argument")]);
        let bx: Box<[u64]> = Box::new([1, 2, 3]);
        assert_eq!(c.bx, bx);
        assert_eq!(c.string, "Data string");
    }

    let mut emitter = Emitter::default();

    emit!(emitter,
        push rsi;
        mov rsi, $pa;
        call_fn primitives(rsi, $pb);
        pop rsi;
    );

    let string = String::from("First Argument");
    let data = Data {
        bx: Box::new([1, 2, 3]),
        string: "Data string",
    };
    let vec = vec![String::from("Second"), String::from("Argument")];
    emit!(emitter,
        call_fn references(ref &string, ref &vec, ref &data);
        ret;
    );

    let code = unsafe { emitter.make_exec() }.unwrap();
    code.execute();
}
