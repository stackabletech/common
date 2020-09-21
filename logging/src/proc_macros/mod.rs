// This needs to be its own inner crate because of Rust limitations with proc_macros
// Currently proc_macros cannot export anything other than the macros itself.
// We need however an export of "slog" because the inner (generated) macros use that
// That's why we wrap this `proc_macros` crate in the outer `stackable_logging` crate
// which in turn has a `pub use ::slog;` export.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, ExprLit, Lit, Token};

// Each log statement consists of two things:
// * A unique code (which must be a number)
// * The log message
struct Log {
    code: ExprLit,
    msg: ExprLit,
}

// This parses two literals separated by a comma:
//     "foo", "bar"
// While this would parse into `ExprLit` just fine we later verify that the literals
// are actually of the types we require.
impl Parse for Log {
    fn parse(input: ParseStream) -> Result<Self> {
        let code: ExprLit = input.parse()?;
        input.parse::<Token![,]>()?;
        let msg = input.parse()?;

        Ok(Log { code, msg })
    }
}

/// This macro can be used to generate a new macro which in turn can be used to log error messages.
///
/// ## Usage
///
/// ```gen_log_error!(1000, "My templated log message looks {how});```
///
/// It accepts two parameters:
/// * An integer literal as a unique error code
/// * A string literal (same syntax as the built in `format!` macro
///
/// This will generate a new macro which accepts (in this case) two parameters:
/// ```log_error_1000!(logger, how="fabulous");
///
/// The first parameter must be an instance of slog's `Logger` trait, the remaining parameters need to be the template variables.
/// If they are not provided it will result in a compile-time error.
#[proc_macro]
pub fn gen_log_error(input: TokenStream) -> TokenStream {
    gen_log_macro(input, "error")
}

// This is the function that takes incoming tokens and generates the new macro
fn gen_log_macro(input: TokenStream, severity: &str) -> TokenStream {
    let Log { code, msg } = parse_macro_input!(input as Log);

    /* TODO: I believe we can generate a map at compile time which includes all error codes used so far
    That way we could abort here with a nice message if the code has been reused
    instead of doing it later in the compilation process */
    let code = match &code.lit {
        Lit::Int(code) => code,
        _ => panic!("[code] needs to be a number"),
    };

    let msg = match &msg.lit {
        Lit::Str(msg) => msg,
        _ => panic!("[msg] needs to be a string literal"),
    };

    // This needs to be formatted outside of the quote! macro because the macro will not
    // make valid identifiers out of concatenated strings. It seems to insert spaces between
    // template expressions. So "log_error_1010" would be expanded to "log_ error _ 1010"
    // which would not be valid Rust and thus not compile.
    let msg = format!("E{code}: {msg}", code = code, msg = msg.value());
    let macro_name = format_ident!("log_{}_{}", severity, code.base10_digits());

    // This is the template for our final generated macro.
    // It takes three variants, all three need a `Logger` instance as the first parameter.
    // Two variants take arguments that are used for the msg string to replace template variables
    // and the last takes an additional tag which is passed verbatim to `slog`.
    let expanded = quote! {
        macro_rules! #macro_name {
            ($log:expr, #$tag:expr, $($arg:tt)*) => {
                ::stackable_logging::slog::error!($log, #$tag, #msg, $($arg)*);
            };
            ($log:expr, $($arg:tt)*) => {
                ::stackable_logging::slog::error!($log, #msg, $($arg)*);
            };
            ($log:expr) => {
                ::stackable_logging::slog::error!($log, #msg);
            }

        }
    };

    TokenStream::from(expanded)
}

// TODO: Write test for the macro, I have no idea yet how to do that properly
