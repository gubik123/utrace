use darling;
use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn;

#[proc_macro]
pub fn trace_here(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(input.into()) {
        Ok(v) => v,
        Err(e) => {
            return proc_macro::TokenStream::from(Error::from(e).write_errors());
        }
    };

    let args = match TraceAttrs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return proc_macro::TokenStream::from(e.write_errors());
        }
    };

    let entry_hash = hash(&format!("Entry_{:?}", Span::call_site()));
    let exit_hash = hash(&format!("Exit_{:?}", Span::call_site()));

    let uniqe_name_entry = &format!("{}", &entry_hash);
    let uniqe_name_exit = &format!("{}", &exit_hash);

    let trace_body_gen;

    if let Some(trace_each_nth_count) = args.trace_each_nth_count {
        trace_body_gen = quote!(

            let tracer = utrace::Tracer::new(
                Some({
                    #[link_section = "_trace_point"]
                    #[export_name=concat!("enter_", module_path!(), "_", line!(), "_", column!(), "_", #uniqe_name_entry)]
                    static ENTRY_ID_HOLDER: u8 = 0;
                    &ENTRY_ID_HOLDER as *const u8 as u8
                }),
                Some({
                    #[link_section = "_trace_point"]
                    #[export_name=concat!("exit_", module_path!(), "_", line!(), "_", column!(), "_",  #uniqe_name_exit)]
                    static END_ID_HOLDER: u8 = 0;
                    &END_ID_HOLDER as *const u8 as u8
                }),
                utrace::tracer::SkipConfig::Skip {
                    counter: {  static mut TRACE_COUNTER: u32 = 0;
                                unsafe {&mut TRACE_COUNTER}},
                    limit: #trace_each_nth_count,
                },
            );
        )
    } else {
        trace_body_gen = quote!(
            static mut TRACE_COUNTER: u32 = 0;

            let tracer = utrace::Tracer::new(
                Some({
                    #[link_section = "_trace_point"]
                    #[export_name=concat!("enter_", module_path!(), "_", line!(), "_", column!(), "_", #uniqe_name_entry)]
                    static ENTRY_ID_HOLDER: u8 = 0;
                    &ENTRY_ID_HOLDER as *const u8 as u8
                }),
                Some({
                    #[link_section = "_trace_point"]
                    #[export_name=concat!("exit_", module_path!(), "_", line!(), "_", column!(), "_",  #uniqe_name_exit)]
                    static END_ID_HOLDER: u8 = 0;
                    &END_ID_HOLDER as *const u8 as u8
                }),
                utrace::tracer::SkipConfig::NoSkip,
            );
        )
    }

    trace_body_gen.into()
}

#[proc_macro_attribute]
pub fn trace(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast: syn::ItemFn = syn::parse(input).expect("Failed to parse input as a function");
    let attr: TokenStream = attr.into();

    let head_ident = &ast.sig;
    let fn_vis = &ast.vis;
    let body = &ast.block;

    let expanded = quote! {
        #fn_vis #head_ident {
        let mut body_future = core::pin::pin!(async move #body);
            core::future::poll_fn(|cx| {
            utrace::trace_here!(#attr);
            use core::future::Future;
            body_future.as_mut().poll(cx)}).await
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn default_transport(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let body: syn::ItemFn =
        syn::parse(input).expect("#[utrace::default_transport] should be applied to a function");

    quote! {
        #[export_name = "__utrace_default_transport_write"]
        #body
    }
    .into()
}

#[proc_macro_attribute]
pub fn timestamp(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let body: syn::ItemFn =
        syn::parse(input).expect("#[utrace::timestamp] should be applied to a function");

    quote! {
        #[export_name = "__utrace_timestamp_function"]
        #body
    }
    .into()
}

#[derive(Debug, FromMeta)]
struct TraceAttrs {
    #[darling(default)]
    trace_each_nth_count: Option<u32>,
}

fn hash(string: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&string, &mut hasher);
    std::hash::Hasher::finish(&hasher)
}
