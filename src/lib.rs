//! ![Crates.io](https://img.shields.io/crates/l/axum-derive-error) ![Crates.io](https://img.shields.io/crates/v/axum-derive-error)
//!
//! Proc macro to derive IntoResponse for error types for use with axum.
//!
//! Your error type just needs to implement Error (Snafu or thiserror could be useful here), IntoResponse and Debug will be derived for you.
//! By default errors will return a 500 response, but this can be specified with the `#[status = ...]` attribute.
//!
//! ## Example:
//! ```rust
//! use std::{error::Error, fmt::Display};
//! use axum_derive_error::ErrorResponse;
//! use axum::http::StatusCode;
//!
//! #[derive(ErrorResponse)]
//! pub enum CreateUserError {
//!     /// No status provided, so this will return a 500 error.
//!     /// All 5xx errors will not display their message to the user, but will produce a tracing::error log
//!     InsertUserToDb(sqlx::Error),
//!
//!     /// 422 returned as the response, the display implementation will be used as a message for the user
//!     #[status(StatusCode::UNPROCESSABLE_ENTITY)]
//!     InvalidBody(String),
//! }
//!
//! impl Error for CreateUserError {
//!     fn source(&self) -> Option<&(dyn Error + 'static)> {
//!         match self {
//!             Self::InsertUserToDb(source) => Some(source),
//!             Self::InvalidBody(_) => None,
//!         }
//!     }
//! }
//!
//! impl Display for CreateUserError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         match self {
//!             Self::InsertUserToDb(source) => write!(f, "failed to insert user into the database"),
//!             Self::InvalidBody(message) => write!(f, "body is invalid: {message}"),
//!         }
//!     }
//! }
//! ```
//!
//! ## License
//!
//! Licensed under either of
//!
//!  * Apache License, Version 2.0
//!    ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
//!  * MIT license
//!    ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
//!
//! at your option.
//!
//! ## Contribution
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.

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
            fn into_response(self) -> ::axum::response::Response {
                let status = self.status_code();
                let mut error_message = self.to_string();

                if status.is_server_error() {
                    ::tracing::error!(error_message, error_details = ?self, "internal server error");
                    error_message = "Internal server error".to_string()
                }

                let body = ::axum::Json(::serde_json::json!({
                    "code": status.as_u16(),
                    "error": error_message,
                }));

                ::axum::response::IntoResponse::into_response((status, body))
            }
        }

        impl ::std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                writeln!(f, "{}\n", self)?;
                let mut current = ::std::error::Error::source(self);
                while let Some(cause) = current {
                    writeln!(f, "Caused by:\n\t{}", cause)?;
                    current = ::std::error::Error::source(cause);
                }
                Ok(())
            }
        }
    }
}
