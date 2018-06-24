///
/// A simple internal logger enabled by setting the MOCKITO_DEBUG environment variable.
/// Works pretty much like the `println!` macro.
///
macro_rules! debug {
    ($($arg:tt)+) => ({
        if ::std::env::var("MOCKITO_DEBUG").is_ok() {
            println!("\n### {}", format_args!($($arg)+));
        }
    });
}
