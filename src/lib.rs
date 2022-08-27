use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput};

#[proc_macro_derive(ErrorResponse, attributes(status))]
pub fn derive_error_response(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let ident = input.ident;

    match input.data {
        Data::Union(_) => panic!("cannot derive ErrorResponse for unions"),
        Data::Struct(_) => panic!("cannot derive ErrorResponse for structs yet"),
        Data::Enum(enum_data) => derive_error_response_for_enum(ident, enum_data).into(),
    }
}

fn derive_error_response_for_enum(ident: Ident, enum_data: DataEnum) -> TokenStream {
    let status_codes = enum_data.variants.into_iter().map(|variant| {
        let variant_name = variant.ident;
        let attr = variant.attrs.into_iter().find(|attr| {
            attr.path
                .get_ident()
                .map(|ident| *ident == "status")
                .unwrap_or_default()
        });
        let match_fields = match variant.fields {
            syn::Fields::Named(_) => quote!({..}),
            syn::Fields::Unnamed(fields) => {
                let fields = fields.unnamed.into_iter().map(|_| quote!(_));
                quote!{
                    (#(#fields,)*)
                }
            },
            syn::Fields::Unit => quote! {},
        };
        match attr {
            Some(attr) => {
                let status = attr.tokens;
                quote! {
                    Self::#variant_name #match_fields => {
                        #[allow(unused_parens)]
                        #status
                    }
                }
            },
            None => {
                quote! { Self::#variant_name #match_fields => ::axum::http::StatusCode::INTERNAL_SERVER_ERROR }
            }
        }
    });

    quote! {
        impl #ident {
            fn status_code(&self) -> ::axum::http::StatusCode {
                match self {
                    #(#status_codes,)*
                }
            }
        }

        impl ::axum::response::IntoResponse for #ident {
            fn into_response(self) -> Response {
                let status = self.status_code();
                let mut error_message = self.to_string();

                if status.is_server_error() {
                    tracing::error!(error_message, error_details = ?self, "internal server error");
                    error_message = "Internal server error".to_string()
                }

                let body = Json(json!({
                    "code": status.as_u16(),
                    "error": error_message,
                }));

                (status, body).into_response()
            }
        }

        impl ::std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                writeln!(f, "{}\n", self)?;
                let mut current = ::std::error::Error::source(self);
                while let Some(cause) = current {
                    writeln!(f, "Caused by:\n\t{}", cause)?;
                    current = cause.source();
                }
                Ok(())
            }
        }
    }
}
