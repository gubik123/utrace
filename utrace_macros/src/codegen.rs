use anyhow::{Context, Result};
use darling::FromMeta;
use proc_macro::Span;
use proc_macro2::TokenStream;
use quote::quote;
use utrace_parser::trace_point::{TracePointInfo, TracePointKind, TracePointPairKind};

fn location_hash() -> u64 {
    let loc_str: String = format!("{:?}", Span::call_site());
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&loc_str, &mut hasher);
    std::hash::Hasher::finish(&hasher)
}

fn trace_point_definition(
    kind: TracePointKind,
    name: Option<String>,
    comment: Option<String>,
    skip: Option<u32>,
) -> TokenStream {
    let tp = TracePointInfo {
        kind,
        name,
        comment,
        skip,
        id: location_hash(),
    };

    let tp_str = tp.to_escaped_string();

    quote! {{
        #[link_section = "utrace_trace_points"]
        #[export_name=#tp_str]
        static ENTRY_ID_HOLDER: u8 = 0;
        &ENTRY_ID_HOLDER as *const u8 as u8
    }}
}

fn tracer_instantiation(
    tracer_kind: TracePointPairKind,
    name: Option<String>,
    comment: Option<String>,
    skip: Option<u32>,
    enable_entry: bool,
    enable_exit: bool,
) -> TokenStream {
    let enter_kind = tracer_kind.enter_point();
    let exit_kind = tracer_kind.exit_point();

    let entry_def = if enable_entry {
        let tpd = trace_point_definition(enter_kind, name.clone(), comment.clone(), skip);
        quote! { Some(#tpd) }
    } else {
        quote! { None }
    };

    let exit_def = if enable_exit {
        let tpd = trace_point_definition(exit_kind, name, comment, skip);
        quote! {Some(#tpd) }
    } else {
        quote! { None }
    };

    let skip_def = if let Some(skip) = skip {
        quote! { utrace::tracer::SkipConfig::Skip {
            counter: {  static mut TRACE_COUNTER: u32 = 0;
                        unsafe {&mut TRACE_COUNTER}},
            limit: #skip,
         }
        }
    } else {
        quote! { utrace::tracer::SkipConfig::NoSkip }
    };

    let tracer_id = syn::Ident::from_string(&format!("_utrace_tracer_{}", location_hash()))
        .expect("Internal problem during tracer instantiation site generation");

    quote! {
        let #tracer_id = utrace::tracer::Tracer::new(#entry_def, #exit_def, #skip_def)
    }
}

pub fn transform_async_fn(
    name: Option<String>,
    attrs: crate::FnAttributesMeta,
    body: TokenStream,
) -> TokenStream {
    let trace_poll = !attrs.noenter_poll || !attrs.noexit_poll;
    let trace_inst = !attrs.noenter_fn || !attrs.noexit_fn;

    let body = if trace_poll {
        let poll_tracer = tracer_instantiation(
            TracePointPairKind::AsyncPoll,
            name.clone(),
            attrs.comment.clone(),
            attrs.skip,
            !attrs.noenter_poll,
            !attrs.noexit_poll,
        );
        quote! {
            let mut utrace_instrumented_body = core::pin::pin!(async move #body);
            core::future::poll_fn(|cx| {
                #poll_tracer;
               core::future::Future::poll(utrace_instrumented_body.as_mut(), cx)
            }).await
        }
    } else {
        body
    };

    if trace_inst {
        let inst_tracer = tracer_instantiation(
            TracePointPairKind::AsyncInstantiation,
            name,
            attrs.comment,
            attrs.skip,
            !attrs.noenter_fn,
            !attrs.noexit_fn,
        );

        quote! {
            #inst_tracer;
            {
                #body
            }
        }
    } else {
        body
    }
}

pub fn transform_sync_fn(
    name: Option<String>,
    attrs: crate::FnAttributesMeta,
    body: TokenStream,
) -> TokenStream {
    let trace_fn = !attrs.noenter_fn || !attrs.noexit_fn;

    if trace_fn {
        let inst_tracer = tracer_instantiation(
            TracePointPairKind::AsyncInstantiation,
            name,
            attrs.comment,
            attrs.skip,
            !attrs.noenter_fn,
            !attrs.noexit_fn,
        );

        quote! {
            #inst_tracer;
            {
                #body
            }
        }
    } else {
        body
    }
}
