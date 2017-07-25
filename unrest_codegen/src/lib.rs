#![feature(type_ascription)]
#![feature(proc_macro)]
#![recursion_limit="128"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;

use std::collections::HashMap;
use proc_macro::TokenStream;
use syn::*;
use quote::ToTokens;

enum Method {
    Get,
    Post,
    Put,
    Delete,
}

struct HandlerAttributes {
    method: Method,
    path: String,
    named_path_segments: Vec<String>,
    data: Option<String>,
}

struct HandlerImpl {
    name: Ident,
    vis: Visibility,
    ha: HandlerAttributes,
    fn_args_str: HashMap<String, String>,
    block: Block,
}

#[proc_macro_attribute]
pub fn handler(attribute: TokenStream, function: TokenStream) -> TokenStream {
    let ha = parse_attribute(attribute);

    let Item { node, .. } = syn::parse(function).unwrap();
    let item_fn = match node {
        ItemKind::Fn(item) => item,
        _ => panic!("handler attr can only be used on functions"),
    };

    let fn_args_str = extract_fn_args(&*item_fn.decl);

    validate_args(&ha, &fn_args_str);

    let gen = impl_handler(HandlerImpl {
        name: item_fn.ident,
        vis: item_fn.vis,
        block: *item_fn.block,
        ha,
        fn_args_str,
    });

    gen.into()
}

fn parse_attribute(ts: TokenStream) -> HandlerAttributes {
    let args_str = ts.to_string();
    let args_trimmed = args_str.trim_matches(|c| c == ')' || c == '(').trim();
    let derive_input = format!("#[{}] struct Dummy;", args_trimmed)
        .parse()
        .unwrap();

    let derive: DeriveInput = syn::parse(derive_input).unwrap();
    let attr = derive.attrs.into_iter().next().expect(
        "expected some attributes",
    );
    let meta = match attr.meta_item().unwrap() {
        MetaItem::List(list) => list,
        _ => panic!("invalid attributes"),
    };

    let method = Method::parse(meta.ident.as_ref());

    let mut meta_attr_iter = meta.nested.into_iter();
    let path_nested_meta = meta_attr_iter
        .next()
        .expect("expected path attr")
        .into_item();

    let path_lit = match path_nested_meta {
        NestedMetaItem::Literal(lit) => lit,
        _ => panic!("expected path attr as str literal"),
    };
    let path = path_lit
        .to_string()
        .trim_matches(|c| c == '"' || c == '"')
        .trim()
        .to_string();

    let named_path_segments = extract_named_path_segments(&path);

    let mut data = None;

    for nested_meta_item in meta_attr_iter {
        let item = nested_meta_item.into_item();
        let nv = match item {
            NestedMetaItem::MetaItem(MetaItem::NameValue(nv)) => nv,
            _ => panic!("expect key-value attr after path"),
        };

        match nv.ident.as_ref() {
            "data" => {
                data = Some(
                    nv.lit
                        .to_string()
                        .trim_matches(|c| c == '"')
                        .trim()
                        .to_string(),
                )
            }
            i => panic!("unknown key '{}' in args", i),
        }
    }


    HandlerAttributes {
        method,
        path,
        named_path_segments,
        data,
    }
}

fn extract_named_path_segments(i: &str) -> Vec<String> {
    let mut out = Vec::new();
    for s in i.split('/') {
        if s.len() > 0 && s.as_bytes()[0] == b':' {
            out.push(s[1..].to_string())
        }
    }
    out
}

fn extract_fn_args(decl: &FnDecl) -> HashMap<String, String> {
    let mut out = HashMap::new();

    for fn_arg in decl.inputs.iter().map(|d| d.into_item()) {
        match *fn_arg {
            FnArg::Captured(ArgCaptured { ref pat, ref ty, .. }) => {
                out.insert(pat.into_tokens().to_string(), ty.into_tokens().to_string());
            }
            _ => panic!("handler arguments must be unignored non self referencing"),
        }
    }
    out
}

fn validate_args(ha: &HandlerAttributes, fn_args: &HashMap<String, String>) {
    let state_arg_count = fn_args.iter().fold(0, |acc, (_, v)| if v.starts_with(
        "State <",
    ) && v.ends_with(">")
    {
        acc + 1
    } else {
        acc
    });

    assert_eq!(
        ha.named_path_segments.len() + state_arg_count + if ha.data.is_some() { 1 } else { 0 },
        fn_args.len(),
        "expected fn args count be equal to attr count"
    );
    for segment in &ha.named_path_segments {
        fn_args.get(segment).expect(
            format!(
                "all segments must be present in fn arguments, but '{}' is missing",
                segment
            ).as_str(),
        );
    }
    if let Some(ref data) = ha.data {
        fn_args.get(data).expect(&format!(
            "no data arg '{}' in fn args",
            data
        ));
    }
}


fn impl_handler(hi: HandlerImpl) -> quote::Tokens {
    let struct_name: Ident = format!("HandlerStruct_{}", hi.name.as_ref()).into();
    let factory_fn_name = hi.name;
    let mod_name: Ident = format!("___mod_handler_{}", hi.name.as_ref()).into();
    let method_ident: Ident = hi.ha.method.as_name_str().into();
    let block = hi.block;
    let path = hi.ha.path.clone();
    let vis = hi.vis;

    let (args_parser_impl, args_applier, handler_args, data_fut, state_fut) =
        impl_args_parser(hi.ha, hi.fn_args_str);


    quote! {
        #[allow(unused_imports)]
        mod #mod_name {
            use unrest::{Handler, Request, Response, Params, Responder, Error, from_data_req, Container, State};
            use futures::{Future, IntoFuture};
            use super::*;

            #[allow(non_camel_case_types)]
            #[allow(dead_code)]
            struct #struct_name {}

            #[allow(dead_code)]
            pub(super) fn #factory_fn_name() -> Box<Handler + 'static> {
                Box::new(#struct_name {})
            }

            fn __handler(#handler_args) -> impl Future<Item=impl Responder, Error=Error> {
                #block
            }
        
            #args_parser_impl

            impl Handler for #struct_name {
                #[allow(unused_variables)]
                fn handle(&self, req: Request, p: Params, state: Container) -> Box<Future<Item = Response, Error = Error>> {
                    let resp = __parse_args(p).into_future()
                    .and_then(|args| {
                        #data_fut
                    })
                    .and_then(move |(args, data)| {
                        Ok((args, data, (#state_fut)))
                    })
                    .and_then(|(args, data, state_args)| {
                        __handler(#args_applier)
                    }).and_then(|r| {
                        Ok(r.respond())
                    });
                    
                    Box::new(resp)
                }
                
                fn path(&self) -> &'static str {
                    #path
                }
                fn method(&self) -> ::hyper::Method {
                    ::hyper::Method::#method_ident
                }
            }
        }
        #vis fn #factory_fn_name() -> Box<::unrest::Handler + 'static> {
            self::#mod_name::#factory_fn_name()
        }
        
    }
}


fn impl_args_parser(
    ha: HandlerAttributes,
    fn_args_str: HashMap<String, String>,
) -> (quote::Tokens, quote::Tokens, quote::Tokens, quote::Tokens, quote::Tokens) {
    use quote::Tokens;

    let mut body_tokens = Tokens::new();
    let mut ty_tokens = Tokens::new();
    let mut apply_tokens = Tokens::new();
    let mut handler_args_tokens = Tokens::new();
    let mut data_fut = Tokens::new();
    let mut state_fut = Tokens::new();


    for (i, name) in ha.named_path_segments.iter().enumerate() {
        let ty: Ident = fn_args_str[name].clone().into();
        let tok =
            quote! {
            params
                        .find(#name)
                        .ok_or_else(|| Error::ParamNotFound(#name))?
                        .parse::<#ty>()
                        .map_err(|e| { let e: Error = e.into(); e })?,
        };

        let ty_tok =
            quote!{
            #ty,  
        };

        let apply_tok: proc_macro2::TokenStream = format!("args.{},", i).parse().unwrap();

        let h_arg_tok: proc_macro2::TokenStream = format!("{}: {},", name, ty).parse().unwrap();

        body_tokens.append_tokens(tok);
        ty_tokens.append_tokens(ty_tok);
        apply_tokens.append_tokens(apply_tok);
        handler_args_tokens.append_tokens(h_arg_tok);
    }

    if let Some(name) = ha.data {
        let ty: proc_macro2::TokenStream = fn_args_str[&name].parse().unwrap();

        let fut_tok =
            quote! {
            from_data_req(req).and_then(move |data| {
                Ok((args, data))
            })
        };

        let apply_tok: proc_macro2::TokenStream = "data,".parse().unwrap();
        let h_arg_tok: proc_macro2::TokenStream = format!("{}: {},", name, ty).parse().unwrap();

        apply_tokens.append_tokens(apply_tok);
        handler_args_tokens.append_tokens(h_arg_tok);
        data_fut.append_tokens(fut_tok);
    } else {
        let fut_tok =
            quote! {
            Ok((args, ()))
        };
        data_fut.append_tokens(fut_tok);
    }

    let mut state_idx = 0;
    for (name_str, ty_str) in fn_args_str {
        if ty_str.starts_with("State <") && ty_str.ends_with(">") {
            let h_arg_tok: proc_macro2::TokenStream =
                format!("{}: {},", name_str, ty_str).parse().unwrap();
            let apply_tok: proc_macro2::TokenStream =
                format!("state_args.{},", state_idx).parse().unwrap();
            let ty: proc_macro2::TokenStream = ty_str[7..ty_str.len() - 1].parse().unwrap();
            let get_state_tok =
                quote! {
                state.get::<#ty>()
                .ok_or_else(|| 
                    Error::StateNotFound(format!("{}: {}", #name_str, #ty_str))
                )?,
            };
            handler_args_tokens.append_tokens(h_arg_tok);
            apply_tokens.append_tokens(apply_tok);
            state_fut.append_tokens(get_state_tok);
            state_idx += 1;
        }
    }



    let parse_args =
        quote! {
        #[allow(unused_variables)]
        fn __parse_args(params: Params) -> Result<(#ty_tokens), Error> {
            let args = (
                #body_tokens
            );

            Ok(args)
        }
    };

    (
        parse_args,
        apply_tokens,
        handler_args_tokens,
        data_fut,
        state_fut,
    )
}

impl Method {
    fn as_name_str(&self) -> &'static str {
        match *self {
            Method::Get => "Get",
            Method::Put => "Put",
            Method::Post => "Post",
            Method::Delete => "Delete",

        }
    }

    fn parse(s: &str) -> Method {
        match s {
            "get" => Method::Get,
            "post" => Method::Post,
            "put" => Method::Put,
            "delete" => Method::Delete,
            n => panic!("unknown method: {}", n),
        }
    }
}
