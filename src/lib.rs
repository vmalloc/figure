#![deny(warnings)]
#![deny(clippy::dbg_macro, clippy::todo, clippy::unimplemented)]
mod config;
mod config_loader;
mod layer;
#[cfg(test)]
mod tests;

#[cfg(doctest)]
mod test_readme {
    macro_rules! external_doc_test {
        ($x:expr) => {
            #[doc = $x]
            extern "C" {}
        };
    }

    external_doc_test!(include_str!("../README.md"));
}

pub use config::Config;
