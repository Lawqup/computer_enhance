#![allow(unused)]

extern crate proc_macro;
use proc_macro::TokenStream as TS;
use proc_macro2::Span;
use quote::ToTokens;

#[cfg(feature = "profile")]
use quote::quote;

use syn::{
    parse::{Parse, ParseStream},
    Block, Expr, ExprLit, Ident, Lit, LitInt, LitStr,
};

#[cfg(feature = "profile")]
use syn::{parse2, parse_macro_input, ItemFn};

#[cfg(feature = "profile")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "profile")]
static COUNTER: AtomicUsize = AtomicUsize::new(1);

struct InstrumentArgs {
    name: Option<String>,
    bytes_processed: Option<Expr>,
    block: Option<Block>,
}

enum InstrumentArg {
    Name(String),
    BytesProcessed(Expr),
    Block(Block),
}

impl Parse for InstrumentArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let res = if lookahead.peek(LitStr) {
            let name_lit = input.parse::<LitStr>()?;
            Self::Name(name_lit.value())
        } else if lookahead.peek(Ident) {
            Self::BytesProcessed(input.parse::<Expr>()?)
        } else {
            Self::Block(input.parse::<Block>()?)
        };

        Ok(res)
    }
}

impl Parse for InstrumentArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args_parsed =
            syn::punctuated::Punctuated::<InstrumentArg, syn::Token![,]>::parse_terminated(input)
                .unwrap();

        let mut name = None;
        let mut bytes_processed = None;
        let mut block = None;
        for arg in args_parsed {
            match arg {
                InstrumentArg::Name(n) => name = Some(n),
                InstrumentArg::BytesProcessed(expr) => bytes_processed = Some(expr),
                InstrumentArg::Block(b) => block = Some(b),
            }
        }

        Ok(InstrumentArgs {
            name,
            bytes_processed,
            block,
        })
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
                let _handle = ::profiler::ProfiledBlock::new(#timer_name, #curr_index, 0);

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

#[proc_macro]
pub fn instr(item: TS) -> TS {
    let input: InstrumentArgs = syn::parse2(item.into()).unwrap();

    let block = input.block;
    #[cfg(feature = "profile")]
    {
        let timer_name = input.name.unwrap_or("anonymous_block".to_string());
        let bytes_processed = input.bytes_processed.unwrap_or(Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new("0", Span::call_site())),
        }));

        let curr_index = get_and_increment_counter();

        quote! {
            {
                let _handle = ::profiler::ProfiledBlock::new(#timer_name, #curr_index, #bytes_processed as usize);

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
