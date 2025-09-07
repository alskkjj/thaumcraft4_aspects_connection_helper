use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub(crate) enum T4ACHError {
    #[snafu(display("generic io error at {err_loc}"), visibility(pub))]
    Io {
        source: std::io::Error,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        err_loc: snafu::Location,
    },

    #[snafu(display("parsing recipes failed at line {line_number}."), visibility(pub))]
    ParsingRecipes {
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        err_loc: snafu::Location,
        line_number: usize,
    },

    #[snafu(display("Function Domain error"), visibility(pub))]
    Math {
        source: crate::math::MathError,
        #[snafu(implicit)]
        err_loc: snafu::Location,
        backtrace: snafu::Backtrace,
    },

    #[snafu(visibility(pub))]
    Database {
        #[snafu(implicit)]
        err_loc: snafu::Location,
        backtrace: snafu::Backtrace,
        source: crate::dao::Errors,
    },

    #[snafu(visibility(pub))]
    ElementNotFound {
        #[snafu(implicit)]
        err_loc: snafu::Location,
        backtrace: snafu::Backtrace,
        element_name: String,
        context: String,
    },
}

pub(crate) type Result<T> = std::result::Result<T, T4ACHError>;

