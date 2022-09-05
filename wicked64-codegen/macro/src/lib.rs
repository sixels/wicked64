mod addressing;
mod emitter;
mod instruction;
mod token;

use instruction::Instruction;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, token::Comma};
use token::Identfier;

struct Instructions(Vec<Instruction>);

impl Parse for Instructions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut instructions = Vec::new();
        while !input.is_empty() {
            instructions.push(input.parse()?);
        }
        Ok(Self(instructions))
    }
}
struct Emit {
    buffer: Identfier,
    instructions: Instructions,
}

impl Parse for Emit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let buffer = input.parse()?;
        input.parse::<Comma>()?;

        Ok(Self {
            buffer,
            instructions: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn emit(tokens: TokenStream) -> TokenStream {
    let Emit {
        buffer,
        instructions,
    } = parse_macro_input!(tokens as Emit);

    let mut src = quote! {
        use w64_codegen::prelude::*;
        use w64_codegen::macro_internals::_emit_instructions;

        fn assert_sized<T: Sized>(_: &T) {}

        let buf = &mut #buffer;
    };
    src.extend(instructions.0.into_iter().map(emitter::emit));

    quote!({ #src }).into()
}

#[proc_macro]
pub fn _emit_instructions(tokens: TokenStream) -> TokenStream {
    let instructions = parse_macro_input!(tokens as Instructions);
    (instructions.0)
        .into_iter()
        .map(emitter::emit)
        .collect::<proc_macro2::TokenStream>()
        .into()
}
