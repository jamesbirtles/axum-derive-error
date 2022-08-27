# axum-derive-error

![Crates.io](https://img.shields.io/crates/l/axum-derive-error) ![Crates.io](https://img.shields.io/crates/v/axum-derive-error)

Proc macro to derive IntoResponse for error types for use with axum.

Your error type just needs to implement Error (Snafu or thiserror could be useful here), IntoResponse and Debug will be derived for you.
By default errors will return a 500 response, but this can be specified with the `#[status = ...]` attribute.

## Example:
```rust
use std::{error::Error, fmt::Display};
use axum_derive_error::ErrorResponse;
use axum::http::StatusCode;

#[derive(ErrorResponse)]
pub enum CreateUserError {
    /// No status provided, so this will return a 500 error.
    /// All 5xx errors will not display their message to the user, but will produce a tracing::error log
    InsertUserToDb(sqlx::Error),

    /// 422 returned as the response, the display implementation will be used as a message for the user
    #[status(StatusCode::UNPROCESSABLE_ENTITY)]
    InvalidBody(String),
}

impl Error for CreateUserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InsertUserToDb(source) => Some(source),
            Self::InvalidBody(_) => None,
        }
    }
}

impl Display for CreateUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsertUserToDb(source) => write!(f, "failed to insert user into the database"),
            Self::InvalidBody(message) => write!(f, "body is invalid: {message}"),
        }
    }
}
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
