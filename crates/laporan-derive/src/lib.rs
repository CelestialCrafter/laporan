use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, Ident, LitStr, Member, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

mod kw {
    syn::custom_keyword!(display);
    syn::custom_keyword!(debug);
}

#[proc_macro_derive(IntoReport, attributes(suggestion, attachment, context))]
pub fn derive_into_report(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match build_into_report_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn build_into_report_impl(input: &DeriveInput) -> Result<TokenStream> {
    let body = match &input.data {
        Data::Struct(data) => {
            let frames = build_frames(&input.attrs)?;
            let bindings: TokenStream = data
                .fields
                .iter()
                .filter_map(|field| {
                    let ident = field.ident.as_ref()?;
                    Some(quote! { let #ident = &self.#ident; })
                })
                .collect();

            quote! {{
                #bindings

                let message = ::laporan::__private::alloc::string::ToString::to_string(&self);
                ::laporan::Report::new(message)
                    #(.with_frame(#frames))*
                    .cast()
            }}
        }
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let enum_ident = &input.ident;
                    let variant_ident = &variant.ident;

                    let bindings = match &variant.fields {
                        Fields::Named(fields) => {
                            let members = fields
                                .named
                                .iter()
                                .filter_map(|field| field.ident.as_ref());
                            quote!({ #(ref #members),* })
                        }
                        Fields::Unnamed(_) => quote!(( .. )),
                        Fields::Unit => TokenStream::new(),
                    };

                    let frames = build_frames(&variant.attrs)?;
                    Ok(quote! {
                        #enum_ident::#variant_ident #bindings => ::laporan::Report::new(message) #(.with_frame(#frames))*
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            quote! {{
                let message = ::laporan::__private::alloc::string::ToString::to_string(&self);
                match self { #(#arms),* }.cast()
            }}
        }
        Data::Union(_) => return Err(unsupported_error(input.span(), "unions")),
    };

    let mut generics = input.generics.clone();
    let ident = &input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote! {
            #ident #ty_generics: ::core::marker::Send + ::core::marker::Sync + 'static + ::core::fmt::Display
        });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::laporan::IntoReport for #ident #ty_generics #where_clause {
            #[track_caller]
            #[allow(unused_variables)]
            fn into_report(self) -> ::laporan::Report<Self> {
                #body
            }
        }
    })
}

fn build_frames(attrs: &[Attribute]) -> Result<Vec<TokenStream>> {
    let mut frames = Vec::new();
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("suggestion") {
            frames.push(build_suggestion(attr)?);
        } else if path.is_ident("attachment") {
            frames.push(build_attachment(attr)?);
        } else if path.is_ident("context") {
            frames.extend(build_context(attr)?);
        }
    }

    Ok(frames)
}

fn unsupported_error(span: Span, feature: impl std::fmt::Display) -> Error {
    Error::new(
        span,
        format_args!("#[derive(IntoReport)] does not support {feature}"),
    )
}

fn build_suggestion(attr: &Attribute) -> Result<TokenStream> {
    let fmt: LitStr = attr.parse_args()?;
    Ok(quote!(::laporan::frame::Suggestion::new(
        format_args!(#fmt)
    )))
}

fn build_attachment(attr: &Attribute) -> Result<TokenStream> {
    let value = attr.parse_args_with(|input: ParseStream| {
        let formatted = Formatted::<Member>::parse(input)?;
        Ok(formatted.mode.render(formatted.value))
    })?;

    Ok(quote!(::laporan::frame::Attachment::new(#value)))
}

fn build_context(attr: &Attribute) -> Result<Vec<TokenStream>> {
    let members = attr.parse_args_with(|stream: ParseStream| {
        struct Mapping {
            source: Ident,
            alias: Option<Ident>,
        }

        impl Parse for Mapping {
            fn parse(input: ParseStream) -> Result<Self> {
                Ok(Self {
                    source: Ident::parse(input)?,
                    alias: if input.lookahead1().peek(Token![=]) {
                        input
                            .parse::<Token![=]>()
                            .and_then(|_| Ident::parse(input))
                            .map(Some)?
                    } else {
                        None
                    },
                })
            }
        }

        Punctuated::<Formatted<Mapping>, Token![,]>::parse_terminated(stream)
    })?;

    let frames = members.into_iter().map(|formatted| {
        let mapping = formatted.value;
        let alias = mapping.alias.as_ref().unwrap_or(&mapping.source);

        let key = LitStr::new(&alias.to_string(), alias.span());
        let value = formatted.mode.render(mapping.source);
        quote! { ::laporan::frame::Context::new(#key, #value) }
    });

    Ok(frames.collect())
}

struct Formatted<T> {
    mode: FormatMode,
    value: T,
}

#[derive(Clone, Copy)]
enum FormatMode {
    Debug,
    Display,
}

impl FormatMode {
    fn render(self, value: impl ToTokens) -> TokenStream {
        let directive = match self {
            Self::Debug => "{:?}",
            Self::Display => "{}",
        };

        quote!(format_args!(#directive, #value))
    }
}

impl<T: Parse> Parse for Formatted<T> {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let mode = if lookahead.peek(kw::debug) {
            input.parse::<kw::debug>().map(|_| FormatMode::Debug)
        } else if lookahead.peek(kw::display) {
            input.parse::<kw::display>().map(|_| FormatMode::Display)
        } else {
            Err(lookahead.error())
        }?;

        let content;
        syn::parenthesized!(content in input);

        Ok(Self {
            mode,
            value: content.parse()?,
        })
    }
}
