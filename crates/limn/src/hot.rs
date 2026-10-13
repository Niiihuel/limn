//! Hot reload: the running window takes the code you just saved.
//!
//! Under `dx serve --hot-patch`, and with the `hot` feature, a change is
//! compiled alone and patched into the running process; the next frame runs
//! it. An application needs three things, and each is a plain call that does
//! nothing without the feature, so nothing has to be written twice:
//!
//! ```no_run
//! # struct State; fn view(_: &State) {} fn request_repaint() {}
//! # let state = State;
//! // 1. First thing in `main`.
//! limn::hot::connect();
//! // 2. Once, when the window exists: paint again when a patch lands.
//! limn::hot::on_patch(request_repaint);
//! // 3. Every frame: build the view through the patch table.
//! let root = limn::hot::call(|| view(&state));
//! ```
//!
//! What cannot be patched, such as a new field on a live struct, restarts
//! the application. [`remember`] and [`restore`] carry its state over that.

/// Whether this build can be patched: the `hot` feature, on a desktop target.
pub const ENABLED: bool = cfg!(all(feature = "hot", not(target_arch = "wasm32")));

/// Connect to `dx serve --hot-patch`. Call it first in `main`, before the
/// window opens. Without `dx` running it finds nobody and goes on.
pub fn connect() {
    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    dioxus_devtools::connect_subsecond();
}

/// Call `f` through the patch table, so that after a patch the new code
/// runs. Wrap what you edit while the window is open: the function that
/// builds the view and the one that applies an action. What `f` calls is
/// patched with it; a function called outside is not.
pub fn call<T>(f: impl FnMut() -> T) -> T {
    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    return subsecond::call(f);
    #[cfg(not(all(feature = "hot", not(target_arch = "wasm32"))))]
    {
        let mut f = f;
        f()
    }
}

/// Run `wake` whenever a patch lands, from the thread that received it. A
/// window that is idle paints nothing until something asks: give it the
/// backend's request for a repaint.
pub fn on_patch(wake: impl Fn() + Send + Sync + 'static) {
    #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
    subsecond::register_handler(std::sync::Arc::new(wake));
    #[cfg(not(all(feature = "hot", not(target_arch = "wasm32"))))]
    let _ = wake;
}

/// Keep `state` for the next launch of the application `name`, so a restart
/// lands where the last run was. Call it after each action. It writes a file
/// in the system's temporary directory, and only in a build that can be
/// patched: a release build keeps nothing.
pub fn remember(name: &str, state: &str) {
    if ENABLED {
        // Best effort: a lost session only costs a click.
        let _ = std::fs::write(session_file(name), state);
    }
}

/// What the last run of `name` left with [`remember`], if this build can be
/// patched and there is any.
pub fn restore(name: &str) -> Option<String> {
    ENABLED.then(|| std::fs::read_to_string(session_file(name)).ok()).flatten()
}

fn session_file(name: &str) -> std::path::PathBuf {
    // A name is not a path: nothing in it may leave the temporary directory.
    let name: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    std::env::temp_dir().join(format!("limn-hot-{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_runs_what_it_is_given_and_returns_its_value() {
        let mut runs = 0;
        assert_eq!(call(|| { runs += 1; runs * 21 }), 21);
        assert_eq!(runs, 1);
    }

    #[test]
    fn a_session_file_stays_in_the_temporary_directory_whatever_the_name() {
        let file = session_file("../../etc/pass wd");
        assert_eq!(file.parent(), Some(std::env::temp_dir().as_path()));
        assert_eq!(file.file_name().unwrap(), "limn-hot-______etc_pass_wd");
    }

    #[test]
    fn a_build_that_cannot_be_patched_keeps_no_session() {
        if !ENABLED {
            remember("limn-hot-test", "state");
            assert_eq!(restore("limn-hot-test"), None);
            assert!(!session_file("limn-hot-test").exists());
        }
    }
}
