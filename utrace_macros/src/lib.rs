use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn;

#[proc_macro]
pub fn trace_here(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let entry_hash = hash(&format!("{:?}", Span::call_site()));
    let exit_hash = hash(&format!("{:?}", Span::call_site()));

    let file = file!();
    let line = line!();

    let uniqe_name_entry = &format!("Entry_{}", &entry_hash);
    let uniqe_name_exit = &format!("Exit_{}", &exit_hash);

    let trace_body_gen = quote!(

        #[link_section="_trace_point"]
        #[export_name=concat!("enter_", #file, "_", #line, "_", #uniqe_name_entry)]

        static ENTRY_ID_HOLDER: u8 = 0;
        let entry_id = &ENTRY_ID_HOLDER as *const u8 as u8;

        #[link_section="_trace_point"]
        #[export_name=concat!("exit_", #file, "_", #line, "_", #uniqe_name_exit)]

        static END_ID_HOLDER: u8 = 0;
        let exit_id = &END_ID_HOLDER as *const u8 as u8;

        let tracer = utrace::Tracer::new(Some(entry_id), Some(exit_id));

    );

    trace_body_gen.into()
}

#[proc_macro_attribute]
pub fn trace(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast: syn::ItemFn = syn::parse(input).expect("Failed to parse input as a function");
    let head_ident = &ast.sig;
    let body = &ast.block;

    let expanded = quote! {
        #head_ident {
        let mut body_future = core::pin::pin!(async move #body);
            core::future::poll_fn(|cx| {
            utrace::trace_here!();
            body_future.as_mut().poll(cx)}).await
        }
    };

    expanded.into()
}

fn hash(string: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&string, &mut hasher);
    std::hash::Hasher::finish(&hasher)
}
