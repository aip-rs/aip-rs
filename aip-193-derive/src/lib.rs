use darling::{FromDeriveInput, FromField, FromVariant, ast};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::{DeriveInput, Expr, Ident, parse_macro_input};

#[derive(Debug)]
enum DomainValue {
    String(String),
    Function(Expr),
}

impl darling::FromMeta for DomainValue {
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(DomainValue::String(value.to_string()))
    }

    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        // Check if this is a string literal expression
        if let Expr::Lit(expr_lit) = expr {
            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                return Ok(DomainValue::String(lit_str.value()));
            }
        }

        // Otherwise treat it as a function path
        Ok(DomainValue::Function(expr.clone()))
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(status), supports(enum_any))]
struct StatusInput {
    ident: Ident,
    data: ast::Data<StatusVariant, ()>,
    domain: DomainValue,
    #[darling(default)]
    into_response: bool,
    #[darling(default = "default_true")]
    use_display: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, FromVariant)]
#[darling(attributes(status))]
struct StatusVariant {
    ident: Ident,
    fields: ast::Fields<StatusField>,
    code: Ident,
    #[darling(default)]
    message: Option<String>,
    #[darling(default)]
    use_display: Option<bool>,
}

#[derive(Debug, FromField)]
#[darling(attributes(status))]
struct StatusField {
    ident: Option<Ident>,
    #[darling(default)]
    metadata: bool,
    #[darling(default)]
    metadata_key: Option<String>,
}

fn get_crate_path() -> proc_macro2::TokenStream {
    if let Ok(found) = crate_name("aip") {
        return match found {
            FoundCrate::Itself => quote!(crate::__private::errors),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident::__private::errors)
            }
        };
    }

    if let Ok(found) = crate_name("aip-193") {
        return match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
        };
    }

    quote!(::aip_193)
}

#[proc_macro_derive(IntoStatus, attributes(status))]
pub fn derive_into_status(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let parsed = match StatusInput::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let expanded = generate_impl(&parsed);
    TokenStream::from(expanded)
}

fn generate_impl(input: &StatusInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let krate = get_crate_path();

    let variants = match &input.data {
        ast::Data::Enum(variants) => variants,
        _ => panic!("IntoStatus only supports enums"),
    };

    let code_arms = generate_code_arms(name, variants, &krate);
    let message_arms = generate_message_arms(name, variants, input.use_display);
    let metadata_arms = generate_metadata_arms(name, variants);

    let domain_impl = match &input.domain {
        DomainValue::String(s) => quote! { #s },
        DomainValue::Function(expr) => quote! { (#expr)() },
    };

    let into_status_impl = quote! {
        impl #krate::__private::IntoStatus for #name {
            fn code(&self) -> #krate::Code {
                match self {
                    #(#code_arms),*
                }
            }

            fn message(&self) -> ::std::string::String {
                match self {
                    #(#message_arms),*
                }
            }

            fn reason(&self) -> &str {
                self.as_ref()
            }

            fn domain(&self) -> &str {
                #domain_impl
            }

            fn metadata(&self) -> #krate::__private::HashMap<::std::string::String, ::std::string::String> {
                match self {
                    #(#metadata_arms),*
                }
            }
        }
    };

    let into_response_impl = if input.into_response {
        generate_into_response_impl(name, &krate)
    } else {
        quote! {}
    };

    quote! {
        #into_status_impl
        #into_response_impl
    }
}

fn get_axum_path() -> proc_macro2::TokenStream {
    if let Ok(found) = crate_name("axum") {
        return match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
        };
    }

    if let Ok(found) = crate_name("axum-core") {
        return match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
        };
    }

    quote!(::axum)
}

fn generate_into_response_impl(
    name: &Ident,
    krate: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let axum = get_axum_path();

    quote! {
        impl #axum::response::IntoResponse for #name {
            fn into_response(self) -> #axum::response::Response {
                use #krate::__private::IntoStatus as _;
                let status = Status::from(self);
                <#krate::__private::Status as #axum::response::IntoResponse>::into_response(status)
            }
        }
    }
}

fn generate_code_arms(
    enum_name: &Ident,
    variants: &[StatusVariant],
    krate: &proc_macro2::TokenStream,
) -> Vec<proc_macro2::TokenStream> {
    variants
        .iter()
        .map(|v| {
            let code = &v.code;
            let pattern = generate_pattern_ignore_fields(enum_name, &v.ident, &v.fields);
            quote! {
                #pattern => #krate::Code::#code
            }
        })
        .collect()
}

fn generate_message_arms(
    enum_name: &Ident,
    variants: &[StatusVariant],
    use_display: bool,
) -> Vec<proc_macro2::TokenStream> {
    variants
        .iter()
        .map(|v| {
            let message_expr = if let Some(template) = &v.message {
                let pattern = generate_pattern(enum_name, v);
                let message = parse_message_template(template, &v.fields);
                quote! { #pattern => #message }
            } else {
                let should_use_display = v.use_display.unwrap_or(use_display);
                let pattern = generate_pattern_ignore_fields(enum_name, &v.ident, &v.fields);
                if should_use_display {
                    quote! { #pattern => ::std::string::ToString::to_string(self) }
                } else {
                    let default_msg = format!("{}", v.ident);
                    quote! { #pattern => #default_msg.to_string() }
                }
            };

            message_expr
        })
        .collect()
}

fn parse_message_template(
    template: &str,
    fields: &ast::Fields<StatusField>,
) -> proc_macro2::TokenStream {
    let field_names: Vec<String> = fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    let mut format_str = String::new();
    let mut args: Vec<proc_macro2::TokenStream> = Vec::new();

    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut field_name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    break;
                }
                field_name.push(chars.next().unwrap());
            }

            if field_names.contains(&field_name) {
                format_str.push_str("{}");
                let field_ident = Ident::new(&field_name, proc_macro2::Span::call_site());
                args.push(quote! { #field_ident });
            } else {
                format_str.push('{');
                format_str.push_str(&field_name);
                format_str.push('}');
            }
        } else {
            format_str.push(c);
        }
    }

    if args.is_empty() {
        quote! { #template.to_string() }
    } else {
        quote! { format!(#format_str, #(#args),*) }
    }
}

fn generate_metadata_arms(
    enum_name: &Ident,
    variants: &[StatusVariant],
) -> Vec<proc_macro2::TokenStream> {
    variants
        .iter()
        .map(|v| {
            let pattern = generate_pattern_for_metadata(enum_name, v);

            let metadata_fields: Vec<_> = v
                .fields
                .iter()
                .filter(|f| f.metadata)
                .filter_map(|f| {
                    let field_name = f.ident.as_ref()?;
                    let key = f
                        .metadata_key
                        .clone()
                        .unwrap_or_else(|| field_name.to_string());
                    Some(quote! {
                        map.insert(#key.to_string(), #field_name.to_string());
                    })
                })
                .collect();

            quote! {
                #pattern => {
                    #[allow(unused_mut)]
                    let mut map = ::std::collections::HashMap::new();
                    #(#metadata_fields)*
                    map
                }
            }
        })
        .collect()
}

fn generate_pattern(enum_name: &Ident, variant: &StatusVariant) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;

    match &variant.fields.style {
        ast::Style::Unit => {
            quote! { #enum_name::#variant_name }
        }
        ast::Style::Struct => {
            let field_names: Vec<_> = variant
                .fields
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            quote! { #enum_name::#variant_name { #(#field_names),* } }
        }
        ast::Style::Tuple => {
            let bindings: Vec<_> = (0..variant.fields.len())
                .map(|i| {
                    let ident = Ident::new(&format!("_{}", i), proc_macro2::Span::call_site());
                    quote! { #ident }
                })
                .collect();
            quote! { #enum_name::#variant_name(#(#bindings),*) }
        }
    }
}

fn generate_pattern_ignore_fields(
    enum_name: &Ident,
    variant_name: &Ident,
    fields: &ast::Fields<StatusField>,
) -> proc_macro2::TokenStream {
    match fields.style {
        ast::Style::Unit => {
            quote! { #enum_name::#variant_name }
        }
        ast::Style::Struct => {
            quote! { #enum_name::#variant_name { .. } }
        }
        ast::Style::Tuple => {
            quote! { #enum_name::#variant_name(..) }
        }
    }
}

fn generate_pattern_for_metadata(
    enum_name: &Ident,
    variant: &StatusVariant,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;

    match &variant.fields.style {
        ast::Style::Unit => {
            quote! { #enum_name::#variant_name }
        }
        ast::Style::Struct => {
            let metadata_fields: Vec<_> = variant
                .fields
                .iter()
                .filter(|f| f.metadata)
                .filter_map(|f| f.ident.as_ref())
                .collect();

            if metadata_fields.is_empty() {
                quote! { #enum_name::#variant_name { .. } }
            } else {
                quote! { #enum_name::#variant_name { #(#metadata_fields),*, .. } }
            }
        }
        ast::Style::Tuple => {
            let has_metadata = variant.fields.iter().any(|f| f.metadata);

            if !has_metadata {
                quote! { #enum_name::#variant_name(..) }
            } else {
                let bindings: Vec<_> = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let binding =
                            Ident::new(&format!("_{}", i), proc_macro2::Span::call_site());
                        if f.metadata {
                            quote! { #binding }
                        } else {
                            quote! { _ }
                        }
                    })
                    .collect();
                quote! { #enum_name::#variant_name(#(#bindings),*) }
            }
        }
    }
}