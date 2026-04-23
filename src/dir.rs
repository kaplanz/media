//! Directory paths.

use std::path::PathBuf;

const NAME: &str = env!("CARGO_PKG_NAME");

macro_rules! path {
    ($($dir:tt)*) => {
        $(
            #[must_use]
            pub fn $dir() -> PathBuf {
                xdir::$dir().map(|p| p.join(NAME)).unwrap_or_default()
            }
        )*
    };
}

path! { config data }
