extern crate proc_macro;
use proc_macro::TokenStream as TS;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse2, parse_macro_input, Block, ItemFn, LitStr,
};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct InstrumentArgs {
    name: Option<String>,
}

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

#[proc_macro_attribute]
pub fn instrument(attr: TS, item: TS) -> TS {
    let input_function: ItemFn = parse2(item.into()).unwrap();

    let args: InstrumentArgs = parse_macro_input!(attr);

    let vis = input_function.vis;
    let name = input_function.sig.ident;
    let arguments = input_function.sig.inputs;
    let output = input_function.sig.output;
    let block  = input_function.block;

    let timer_name = args.name.unwrap_or(name.to_string());
    let curr_index = get_and_increment_counter();

    quote! {
        #vis fn #name(#arguments) #output {
            { 
                let handle = ::profiler::profile_start(#timer_name, #curr_index);

                #block
            }
        }
    }
    .into()
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
    let timer_name = input.args.name.unwrap_or("anonymous_block".to_string());
    let curr_index = get_and_increment_counter();

    quote! {
        {
            let handle = ::profiler::profile_start(#timer_name, #curr_index);

            #block
        }
    }
    .into()
}

fn get_and_increment_counter() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

