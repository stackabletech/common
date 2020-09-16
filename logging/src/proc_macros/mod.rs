use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, ExprLit, Lit, Token};


struct Log {
    code: ExprLit,
    msg: ExprLit,
}

impl Parse for Log {
    fn parse(input: ParseStream) -> Result<Self> {
        let code: ExprLit = input.parse()?;
        input.parse::<Token![,]>()?;
        let msg = input.parse()?;

        Ok(Log {
            code,
            msg
        })
    }
}

#[proc_macro]
pub fn gen_log_error(input: TokenStream) -> TokenStream {
    let Log {
        code,
        msg
    } = parse_macro_input!(input as Log);

    let code = match &code.lit {
        Lit::Int(code) => code,
        _ => panic!("[code] needs to be Not a number")
    };

    let msg = match &msg.lit {
        Lit::Str(msg) => msg,
        _ => panic!("[msg] needs to be a string literal")
    };

    let msg = format!("E{code}: {msg}", code = code, msg = msg.value());
    let macro_name= format_ident!("log_error_{}", code.base10_digits());

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


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
