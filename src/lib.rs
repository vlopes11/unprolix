#![feature(external_doc)]
#![doc(include = "../README.md")]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, parse_quote, Block, Data, DeriveInput, Expr, Field, FieldValue, Fields,
    Ident, Member, Meta, NestedMeta, PathArguments, Stmt, Token, Type, Visibility,
};

fn search_for_attribute(f: &Field, attribute: &str) -> bool {
    let mut attr = false;

    for a in f.attrs.iter() {
        match a.parse_meta().unwrap() {
            Meta::List(l) => {
                l.nested.iter().for_each(|l| match l {
                    NestedMeta::Meta(m) => match m.to_token_stream().into_iter().next().unwrap() {
                        TokenTree::Ident(i) if i == attribute => attr = true,
                        _ => (),
                    },
                    _ => (),
                });
            }
            _ => (),
        }
    }

    attr
}

/// Generate a `pub fn new(...) -> Self` method
///
/// All the attributes will be included as parameters of the `new` function.
///
/// ## Default
///
/// Some attributes who implements [`Default`] may not be required as parameter of the constructor.
///
/// For that, there is the option to use `#[unprolix(default)]`
///
/// ## Expansion
///
/// The following code
///
/// ```ignore
/// #[derive(Constructor)]
/// struct SomeStruct {
///     a: u8,
///     b: u8,
///     #[unprolix(default)]
///     c: u8,
/// }
/// ```
///
/// Expands to
///
/// ```ignore
/// impl SomeStruct {
///     pub fn new(a, b) -> Self {
///         Self {
///             a,
///             b,
///             c: Default::default(),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(Constructor, attributes(unprolix))]
pub fn constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let data = input.data;

    let mut values: Punctuated<FieldValue, Token![,]> = Punctuated::new();
    let args: Punctuated<Field, Token![,]> = match data {
        Data::Struct(syn::DataStruct {
            struct_token: _,
            fields: Fields::Named(f),
            semi_token: _,
        }) => f
            .named
            .into_pairs()
            .filter_map(|mut p| {
                let ident = p.value().ident.as_ref().cloned().unwrap();

                let default = search_for_attribute(p.value(), "default");
                if default {
                    let fv = FieldValue {
                        attrs: vec![],
                        member: Member::Named(ident.clone()),
                        colon_token: Some(<Token![:]>::default()),
                        expr: Expr::Call(syn::parse_str("Default::default()").unwrap()),
                    };
                    values.push(fv);

                    None
                } else {
                    let fv = FieldValue {
                        attrs: vec![],
                        member: Member::Named(ident.clone()),
                        colon_token: None,
                        expr: Expr::Verbatim(ident.to_token_stream()),
                    };
                    values.push(fv);

                    (*p.value_mut()).attrs = vec![];
                    (*p.value_mut()).vis = Visibility::Inherited;
                    (*p.value_mut()).colon_token = None;

                    Some(p)
                }
            })
            .collect(),
        _ => Punctuated::new(),
    };

    let expanded = quote! {
        impl #name {
            pub fn new(#args) -> #name {
                #name {
                    #values
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Generate `pub fn attribute(&self) -> Type { &self.attribute }` functions for every non-public
/// attribute.
///
/// ## Skip
///
/// To skip certain attributes that you don't want to expose, you can use `#[unprolix(skip)]`
///
/// ## Copy
///
/// If your type implements [`Copy`] and references are more expensive than copying, then you can
/// use `#[unprolix(copy)]`
///
/// This is recommended for raw numbers or other simple types
///
/// ## Slice
///
/// Its not a good practice to pass vectors as references. For that, or any type that implements an
/// `T<S, ...> fn as_slice(&self) -> &[S]`, you can use `#[unprolix(as_slice)]`
///
/// ## Expansion
///
/// The following code
///
/// ```ignore
/// #[derive(Getters)]
/// struct SomeStruct {
///     a: HashMap<String, i32>,
///     #[unprolix(copy)]
///     b: u8,
///     #[unprolix(skip)]
///     c: u8,
///     #[unprolix(as_slice)]
///     d: Vec<u8>,
/// }
/// ```
///
/// Expands to
///
/// ```ignore
/// impl SomeStruct {
///     pub fn a(&self) -> &HashMap<String, i32> {
///         &self.a
///     }
///
///     pub fn b(&self) -> u8 {
///         self.b
///     }
///
///     pub fn d(&self) -> &[u8] {
///         self.b.as_slice()
///     }
/// }
/// ```
#[proc_macro_derive(Getters, attributes(unprolix))]
pub fn getters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let data = input.data;

    let block: Block = match data {
        Data::Struct(syn::DataStruct {
            struct_token: _,
            fields: Fields::Named(f),
            semi_token: _,
        }) => f
            .named
            .into_pairs()
            .filter_map(|p| {
                if let Visibility::Public(_) = p.value().vis {
                    None
                } else if search_for_attribute(p.value(), "skip") {
                    None
                } else {
                    Some(p.into_value())
                }
            })
            .fold(syn::parse_str("{}").unwrap(), |mut block, field| {
                let copy = search_for_attribute(&field, "copy");
                let as_slice = search_for_attribute(&field, "as_slice");

                let ident = field.ident.as_ref().cloned().unwrap();
                let ty = field.ty;

                let f: Stmt;

                if copy {
                    f = parse_quote! {
                        pub fn #ident(&self) -> #ty {
                            self.#ident
                        }
                    };
                } else if as_slice {
                    let ty = match &ty {
                        Type::Path(p) => {
                            let v = p.path.segments.iter().next().unwrap().clone();
                            let v = match v.arguments {
                                PathArguments::AngleBracketed(v) => v,
                                _ => panic!("Vector type expected"),
                            };
                            v.args.into_iter().next().unwrap()
                        }
                        _ => panic!("as_slice is expected only for Vec types"),
                    };

                    f = parse_quote! {
                        pub fn #ident(&self) -> &[#ty] {
                            self.#ident.as_slice()
                        }
                    };
                } else {
                    f = parse_quote! {
                        pub fn #ident(&self) -> &#ty {
                            &self.#ident
                        }
                    };
                }

                block.stmts.push(f);

                block
            }),
        _ => syn::parse_str("{}").unwrap(),
    };

    let expanded = quote! {
        impl #name #block
    };

    TokenStream::from(expanded)
}

/// Generate `pub fn attribute(&mut self, v: T) { self.attribute = v; }` functions for every non-public
/// attribute.
///
/// ## Skip
///
/// To skip certain attributes that you don't want to expose, you can use `#[unprolix(skip)]`
///
/// ## Expansion
///
/// The following code
///
/// ```ignore
/// #[derive(Setters)]
/// struct SomeStruct {
///     a: u8,
///     #[unprolix(skip)]
///     b: u8,
/// }
/// ```
///
/// Expands to
///
/// ```ignore
/// impl SomeStruct {
///     pub fn a(&mut self, v: u8) {
///         self.a = v;
///     }
///
///     pub fn a_as_mut(&mut self) -> &mut u8 {
///         &mut self.a
///     }
/// }
/// ```
#[proc_macro_derive(Setters, attributes(unprolix))]
pub fn setters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let data = input.data;

    let block: Block = match data {
        Data::Struct(syn::DataStruct {
            struct_token: _,
            fields: Fields::Named(f),
            semi_token: _,
        }) => f
            .named
            .into_pairs()
            .filter_map(|p| {
                if let Visibility::Public(_) = p.value().vis {
                    None
                } else if search_for_attribute(p.value(), "skip") {
                    None
                } else {
                    Some(p.into_value())
                }
            })
            .fold(syn::parse_str("{}").unwrap(), |mut block, field| {
                let ident = field.ident.as_ref().cloned().unwrap();
                let method: Ident = syn::parse_str(format!("set_{}", ident).as_str()).unwrap();
                let method_as_mut: Ident =
                    syn::parse_str(format!("{}_as_mut", ident).as_str()).unwrap();
                let ty = field.ty;

                block.stmts.push(parse_quote! {
                    pub fn #method(&mut self, v: #ty) {
                        self.#ident = v;
                    }
                });

                block.stmts.push(parse_quote! {
                    pub fn #method_as_mut(&mut self) -> &mut #ty {
                        &mut self.#ident
                    }
                });

                block
            }),
        _ => syn::parse_str("{}").unwrap(),
    };

    let expanded = quote! {
        impl #name #block
    };

    TokenStream::from(expanded)
}
