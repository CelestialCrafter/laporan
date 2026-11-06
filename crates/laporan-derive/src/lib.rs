use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, Ident, Index, LitStr, Member, Result, Token,
    Variant,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

#[proc_macro_derive(
    IntoReport,
    attributes(suggestion, attachment, context, message, format)
)]
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
            let message = build_message(&input.ident, &input.attrs, &data.fields)?;
            let frames = build_frames(&input.attrs, &data.fields)?;
            let bindings: TokenStream = data
                .fields
                .members()
                .map(|member| {
                    let escaped = escape_member(member.clone());
                    quote! { let #escaped = &self.#member; }
                })
                .collect();

            quote! {{
                #bindings

                ::laporan::Report::new(#message)
                    #(.with_frame(#frames))*
                    .cast()
            }}
        }
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let message = build_message(&variant.ident, &variant.attrs, &variant.fields)?;
                    let frames = build_frames(&variant.attrs, &variant.fields)?;
                    let bindings = build_bindings_enum(variant);
                    let enum_ident = &input.ident;
                    let variant_ident = &variant.ident;

                    Ok(quote! {
                        #enum_ident::#variant_ident #bindings => ::laporan::Report::new(#message) #(.with_frame(#frames))*
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            quote! { match self { #(#arms),* }.cast() }
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
            #ident #ty_generics: ::core::marker::Send + ::core::marker::Sync + 'static
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

fn build_message(ident: &Ident, attrs: &[Attribute], fields: &Fields) -> Result<TokenStream> {
    let attr = attrs
        .iter()
        .find(|attr| attr.path().is_ident("message"))
        .ok_or_else(|| Error::new(ident.span(), "type should have the #[message] attribute"))?;
    build_formatted(fields, "message", attr)
}

fn build_frames(attrs: &[Attribute], fields: &Fields) -> Result<Vec<TokenStream>> {
    let mut frames = Vec::new();
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("suggestion") {
            frames.push(build_suggestion(fields, attr)?);
        } else if path.is_ident("attachment") {
            frames.push(build_attachment(attr)?);
        } else if path.is_ident("context") {
            frames.extend(build_context(attr)?);
        }
    }

    Ok(frames)
}

fn build_bindings_enum(variant: &Variant) -> TokenStream {
    let members = variant.fields.members().map(escape_member);
    match &variant.fields {
        Fields::Named(_) => quote! { {#(ref #members),*} },
        Fields::Unnamed(_) => quote! { (#(ref #members),*) },
        Fields::Unit => TokenStream::new(),
    }
}

fn unsupported_error(span: Span, feature: impl std::fmt::Display) -> Error {
    Error::new(
        span,
        format_args!("#[derive(IntoReport)] does not support {feature}"),
    )
}

fn build_suggestion(fields: &Fields, attr: &Attribute) -> Result<TokenStream> {
    let fmt = build_formatted(fields, "suggestion", attr)?;
    Ok(quote!(::laporan::frame::Suggestion::new(#fmt)))
}

fn build_attachment(attr: &Attribute) -> Result<TokenStream> {
    let value = attr.parse_args_with(|input: ParseStream| {
        let mode = match Mode::parse(input) {
            Ok(mode) => {
                input.parse::<Token![:]>()?;
                mode
            }
            Err(_) => Mode::default(),
        };

        Ok(mode.format(escape_member(Member::parse(input)?)))
    })?;

    Ok(quote!(::laporan::frame::Attachment::new(#value)))
}

fn build_context(attr: &Attribute) -> Result<Vec<TokenStream>> {
    let (mode, members) = attr.parse_args_with(|stream: ParseStream| {
        struct Mapping {
            source: Member,
            dest: Option<Member>,
        }

        impl Parse for Mapping {
            fn parse(input: ParseStream) -> Result<Self> {
                Ok(Self {
                    source: Member::parse(input)?,
                    dest: if input.lookahead1().peek(Token![=]) {
                        input
                            .parse::<Token![=]>()
                            .and_then(|_| Member::parse(input))
                            .map(Some)?
                    } else {
                        None
                    },
                })
            }
        }

        let mode = match Mode::parse(stream) {
            Ok(mode) => {
                stream.parse::<Token![:]>()?;
                mode
            }
            Err(_) => Mode::default(),
        };

        let members = Punctuated::<Mapping, Token![,]>::parse_terminated(stream)?;
        Ok((mode, members))
    })?;

    Ok(members
        .into_iter()
        .map(|pair| {
            let (dest, span) = match pair.dest.as_ref().unwrap_or(&pair.source) {
                Member::Named(ident) => (ident.to_string(), ident.span()),
                Member::Unnamed(index) => (index.index.to_string(), index.span),
            };

            let key = LitStr::new(&dest, span);
            let value = mode.format(escape_member(pair.source));
            quote! { ::laporan::frame::Context::new(#key, #value) }
        })
        .collect())
}

fn build_formatted(fields: &Fields, target: &str, attr: &Attribute) -> Result<TokenStream> {
    let fmt: LitStr = attr.parse_args()?;
    let members: Vec<Member> = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            let marked = field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("format"))
                .filter_map(|attr| attr.parse_args::<Ident>().ok())
                .any(|ident| ident == target);

            marked.then(|| match &field.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index {
                    index: index as u32,
                    span: field.span(),
                }),
            })
        })
        .collect();

    let bindings = match fields {
        Fields::Named(_) => quote!(#(#members = #members),*),
        Fields::Unnamed(_) => {
            let members = members.into_iter().map(escape_member);
            quote!(#(#members),*)
        }
        Fields::Unit => TokenStream::new(),
    };

    Ok(quote!({
        use ::std::borrow::Cow;

        let fmt = format_args!(#fmt, #bindings);
        match fmt.as_str() {
            Some(string) => Cow::Borrowed(string),
            None => Cow::Owned(fmt.to_string()),
        }
    }))
}

#[derive(Default, Debug, Clone, Copy)]
enum Mode {
    #[default]
    Debug,
    Display,
}

impl Mode {
    fn format(self, value: impl ToTokens) -> TokenStream {
        match self {
            Self::Debug => quote!(format_args!("{:?}", #value)),
            Self::Display => quote!(#value),
        }
    }
}

impl Parse for Mode {
    fn parse(input: ParseStream) -> Result<Self> {
        mod kw {
            syn::custom_keyword!(display);
            syn::custom_keyword!(debug);
        }

        let lookahead = input.lookahead1();
        if lookahead.peek(kw::debug) {
            input.parse::<kw::debug>().map(|_| Self::Debug)
        } else if lookahead.peek(kw::display) {
            input.parse::<kw::display>().map(|_| Self::Display)
        } else {
            Err(lookahead.error())
        }
    }
}

fn escape_member(member: Member) -> Ident {
    match member {
        Member::Named(ident) => ident,
        Member::Unnamed(index) => Ident::new(&format!("__self_{}", index.index), index.span),
    }
}
