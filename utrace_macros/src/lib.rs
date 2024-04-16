use darling::ast::NestedMeta;
use darling::FromMeta;
use quote::quote;

mod codegen;

#[proc_macro]
pub fn trace_here(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attrs = NestedMeta::parse_meta_list(input.into()).expect("Malformed trace_here! arguments");
    let attrs = FreestandingMeta::from_list(&attrs).expect("Unable to parse trace_here! arguments");

    let ret = codegen::tracer_instantiation(
        utrace_core::trace_point::TracePointPairKind::Generic,
        None,
        attrs.comment,
        attrs.skip,
        !attrs.noenter,
        !attrs.noexit,
    );

    quote! {#ret;}.into()
}

#[proc_macro_attribute]
pub fn trace(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast: syn::ItemFn = syn::parse(input).expect("Failed to parse input as a function");

    let attrs = NestedMeta::parse_meta_list(attr.into()).expect("Malformed attr list");
    let attrs =
        FnAttributesMeta::from_list(&attrs).expect("Unable to parse #[trace] attribute arguments");

    let head_ident = &ast.sig;
    let fn_vis = &ast.vis;
    let body = &ast.block;
    let body = if ast.sig.asyncness.is_some() {
        codegen::transform_async_fn(Some(ast.sig.ident.to_string()), attrs, quote! {#body})
    } else {
        if attrs.skip_poll.is_some() || attrs.noenter_poll || attrs.noexit_poll {
            panic!("Attributes skip_poll, noenter_poll and noexit_poll cannot be applied to non-async functions")
        }
        codegen::transform_sync_fn(Some(ast.sig.ident.to_string()), attrs, quote! {#body})
    };

    let expanded = quote! {
        #fn_vis #head_ident {
            #body
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
struct FnAttributesMeta {
    #[darling(default)]
    comment: Option<String>,
    #[darling(default)]
    noenter_fn: bool,
    #[darling(default)]
    noexit_fn: bool,
    #[darling(default)]
    noenter_poll: bool,
    #[darling(default)]
    noexit_poll: bool,
    #[darling(default)]
    skip: Option<u32>,
    #[darling(default)]
    skip_poll: Option<u32>,
}

#[derive(Debug, FromMeta)]
struct FreestandingMeta {
    #[darling(default)]
    comment: Option<String>,
    #[darling(default)]
    noenter: bool,
    #[darling(default)]
    noexit: bool,
    #[darling(default)]
    skip: Option<u32>,
}
