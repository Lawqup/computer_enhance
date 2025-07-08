use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitInt, LitStr, Token};

struct RepeatAsmInput {
    instruction: LitStr,
    count: LitInt,
}

impl syn::parse::Parse for RepeatAsmInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let instruction = input.parse()?;
        input.parse::<Token![;]>()?;
        let count = input.parse()?;
        Ok(RepeatAsmInput { instruction, count })
    }
}

#[proc_macro]
pub fn repeat_asm(input: TokenStream) -> TokenStream {
    let RepeatAsmInput { instruction, count } = parse_macro_input!(input as RepeatAsmInput);

    let instr_str = instruction.value();
    let count_val = count.base10_parse::<usize>().unwrap();

    let repeated = (0..count_val)
        .map(|_| format!("{}\n", instr_str))
        .collect::<String>();

    // Remove trailing newline
    let repeated = repeated.trim_end();

    quote! {
        #repeated
    }.into()
}
