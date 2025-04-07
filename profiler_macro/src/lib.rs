#![allow(unused)]

extern crate proc_macro;
use proc_macro::TokenStream as TS;
use quote::ToTokens;

#[cfg(feature = "profile")]
use quote::quote;

use syn::{
    parse::{Parse, ParseStream},
    Block, LitStr,
};

#[cfg(feature = "profile")]
use syn::{parse2, parse_macro_input, ItemFn};

#[cfg(feature = "profile")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "profile")]
static COUNTER: AtomicUsize = AtomicUsize::new(1);

struct InstrumentArgs {
    name: Option<String>,
}

#[cfg(feature = "profile")]
impl Parse for InstrumentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(InstrumentArgs { name: None })
        } else {
            let name_lit = input.parse::<LitStr>()?;
            let name = name_lit.value();
            Ok(InstrumentArgs { name: Some(name) })
        }
    }
}

#[cfg(feature = "profile")]
#[proc_macro_attribute]
pub fn instrument(attr: TS, item: TS) -> TS {
    let input_function: ItemFn = parse2(item.into()).unwrap();

    let args: InstrumentArgs = parse_macro_input!(attr);

    let vis = input_function.vis;
    let name = input_function.sig.ident;
    let arguments = input_function.sig.inputs;
    let output = input_function.sig.output;
    let block = input_function.block;

    let timer_name = args.name.unwrap_or(name.to_string());
    let curr_index = get_and_increment_counter();

    quote! {
        #vis fn #name(#arguments) #output {
            {
                let _handle = ::profiler::ProfiledBlock::new(#timer_name, #curr_index);

                #block
            }
        }
    }
    .into()
}

#[cfg(not(feature = "profile"))]
#[proc_macro_attribute]
pub fn instrument(_attr: TS, item: TS) -> TS {
    item
}

struct ItemInstr {
    args: InstrumentArgs,
    block: Block,
}

impl Parse for ItemInstr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let args = if lookahead.peek(LitStr) {
            let name_lit = input.parse::<LitStr>()?;
            InstrumentArgs {
                name: Some(name_lit.value()),
            }
        } else {
            InstrumentArgs { name: None }
        };

        let block = input.parse::<Block>()?;

        Ok(ItemInstr { args, block })
    }
}

#[proc_macro]
pub fn instr(item: TS) -> TS {
    let input: ItemInstr = syn::parse2(item.into()).unwrap();

    let block = input.block;
    #[cfg(feature = "profile")]
    {
        let timer_name = input.args.name.unwrap_or("anonymous_block".to_string());
        let curr_index = get_and_increment_counter();

        quote! {
            {
                let _handle = ::profiler::ProfiledBlock::new(#timer_name, #curr_index);

                #block
            }
        }
        .into()
    }

    #[cfg(not(feature = "profile"))]
    block.into_token_stream().into()
}

#[cfg(feature = "profile")]
fn get_and_increment_counter() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
