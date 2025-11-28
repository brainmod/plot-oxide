/// Performance instrumentation utilities

/// Minimal timing macro for debug builds
/// Usage: timed!("operation name", { code })
#[macro_export]
macro_rules! timed {
    ($name:expr, $block:expr) => {{
        #[cfg(debug_assertions)]
        let _t = std::time::Instant::now();
        let r = $block;
        #[cfg(debug_assertions)]
        eprintln!("{}: {:?}", $name, _t.elapsed());
        r
    }};
}

/// Profile a function scope using puffin
/// Only active when profiling feature is enabled
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!($name);
    };
}

/// Profile a function using puffin
/// Only active when profiling feature is enabled
#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
    };
}
