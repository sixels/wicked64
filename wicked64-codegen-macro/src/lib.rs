mod addressing;
mod emitter;
mod instruction;

use instruction::Instruction;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, Ident, Token};

struct Emit {
    buffer: Ident,
    instructions: Vec<Instruction>,
}

impl Parse for Emit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let buffer = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut instructions = Vec::new();
        while !input.is_empty() {
            instructions.push(input.parse()?);
        }
        Ok(Self {
            buffer,
            instructions,
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
        use wicked64_codegen::prelude::*;
        let buf = &mut #buffer;
    };
    let gen = instructions.into_iter().map(emitter::emit);
    src.extend(gen);

    quote!({ #src }).into()
}
