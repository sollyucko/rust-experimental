#![feature(rustc_private)]

extern crate rustc_middle;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_session;

use std::convert::TryFrom;
use std::env;
use std::str::FromStr;

use hex::FromHexError;
use log::debug;

use rustc_session::CtfeBacktrace;
use rustc_driver::Compilation;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_middle::ty::TyCtxt;

struct MiriCompilerCalls {
    miri_config: miri::MiriConfig,
}

impl rustc_driver::Callbacks for MiriCompilerCalls {
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        compiler.session().abort_if_errors();

        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
            init_late_loggers(tcx);
            let (entry_def_id, _) = tcx.entry_fn(LOCAL_CRATE).expect("no main function found!");
            let mut config = self.miri_config.clone();

            // Add filename to `miri` arguments.
            config.args.insert(0, compiler.input().filestem().to_string());

            if let Some(return_code) = miri::eval_main(tcx, entry_def_id.to_def_id(), config) {
                std::process::exit(
                    i32::try_from(return_code).expect("Return value was too large!"),
                );
            }
        });

        compiler.session().abort_if_errors();

        Compilation::Stop
    }
}

fn init_early_loggers() {
    // Note that our `extern crate log` is *not* the same as rustc's; as a result, we have to
    // initialize them both, and we always initialize `miri`'s first.
    let env = env_logger::Env::new().filter("MIRI_LOG").write_style("MIRI_LOG_STYLE");
    env_logger::init_from_env(env);
    // We only initialize `rustc` if the env var is set (so the user asked for it).
    // If it is not set, we avoid initializing now so that we can initialize
    // later with our custom settings, and *not* log anything for what happens before
    // `miri` gets started.
    if env::var_os("RUSTC_LOG").is_some() {
        rustc_driver::init_rustc_env_logger();
    }
}

fn init_late_loggers(tcx: TyCtxt<'_>) {
    // We initialize loggers right before we start evaluation. We overwrite the `RUSTC_LOG`
    // env var if it is not set, control it based on `MIRI_LOG`.
    // (FIXME: use `var_os`, but then we need to manually concatenate instead of `format!`.)
    if let Ok(var) = env::var("MIRI_LOG") {
        if env::var_os("RUSTC_LOG").is_none() {
            // We try to be a bit clever here: if `MIRI_LOG` is just a single level
            // used for everything, we only apply it to the parts of rustc that are
            // CTFE-related. Otherwise, we use it verbatim for `RUSTC_LOG`.
            // This way, if you set `MIRI_LOG=trace`, you get only the right parts of
            // rustc traced, but you can also do `MIRI_LOG=miri=trace,rustc_mir::interpret=debug`.
            if log::Level::from_str(&var).is_ok() {
                env::set_var(
                    "RUSTC_LOG",
                    &format!("rustc_middle::mir::interpret={0},rustc_mir::interpret={0}", var),
                );
            } else {
                env::set_var("RUSTC_LOG", &var);
            }
            rustc_driver::init_rustc_env_logger();
        }
    }

    // If `MIRI_BACKTRACE` is set and `RUSTC_CTFE_BACKTRACE` is not, set `RUSTC_CTFE_BACKTRACE`.
    // Do this late, so we ideally only apply this to Miri's errors.
    if let Some(val) = env::var_os("MIRI_BACKTRACE") {
        let ctfe_backtrace = match &*val.to_string_lossy() {
            "immediate" => CtfeBacktrace::Immediate,
            "0" => CtfeBacktrace::Disabled,
            _ => CtfeBacktrace::Capture,
        };
        *tcx.sess.ctfe_backtrace.borrow_mut() = ctfe_backtrace;
    }
}

/// Returns the "default sysroot" that Miri will use if no `--sysroot` flag is set.
/// Should be a compile-time constant.
fn compile_time_sysroot() -> Option<String> {
    if option_env!("RUSTC_STAGE").is_some() {
        // This is being built as part of rustc, and gets shipped with rustup.
        // We can rely on the sysroot computation in librustc_session.
        return None;
    }
    // For builds outside rustc, we need to ensure that we got a sysroot
    // that gets used as a default.  The sysroot computation in librustc_session would
    // end up somewhere in the build dir (see `get_or_default_sysroot`).
    // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    Some(match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT")
            .expect("To build Miri without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_owned(),
    })
}

/// Execute a compiler with the given CLI arguments and callbacks.
fn run_compiler(mut args: Vec<String>, callbacks: &mut (dyn rustc_driver::Callbacks + Send)) -> ! {
    // Make sure we use the right default sysroot. The default sysroot is wrong,
    // because `get_or_default_sysroot` in `librustc_session` bases that on `current_exe`.
    //
    // Make sure we always call `compile_time_sysroot` as that also does some sanity-checks
    // of the environment we were built in.
    // FIXME: Ideally we'd turn a bad build env into a compile-time error via CTFE or so.
    if let Some(sysroot) = compile_time_sysroot() {
        let sysroot_flag = "--sysroot";
        if !args.iter().any(|e| e == sysroot_flag) {
            // We need to overwrite the default that librustc_session would compute.
            args.push(sysroot_flag.to_owned());
            args.push(sysroot);
        }
    }

    // Some options have different defaults in Miri than in plain rustc; apply those by making
    // them the first arguments after the binary name (but later arguments can overwrite them).
    args.splice(1..1, miri::miri_default_args().iter().map(ToString::to_string));

    // Invoke compiler, and handle return code.
    let result = rustc_driver::catch_fatal_errors(move || {
        rustc_driver::run_compiler(&args, callbacks, None, None)
    })
    .and_then(|result| result);
    let exit_code = match result {
        Ok(()) => rustc_driver::EXIT_SUCCESS,
        Err(_) => rustc_driver::EXIT_FAILURE,
    };
    std::process::exit(exit_code)
}

fn main() {
    rustc_driver::install_ice_hook();

    // If the environment asks us to actually be rustc, then do that.
    if env::var_os("MIRI_BE_RUSTC").is_some() {
        rustc_driver::init_rustc_env_logger();
        // We cannot use `rustc_driver::main` as we need to adjust the CLI arguments.
        let mut callbacks = rustc_driver::TimePassesCallbacks::default();
        run_compiler(env::args().collect(), &mut callbacks)
    }

    // Init loggers the Miri way.
    init_early_loggers();

    // Parse our arguments and split them across `rustc` and `miri`.
    let mut validate = true;
    let mut stacked_borrows = true;
    let mut check_alignment = true;
    let mut communicate = false;
    let mut ignore_leaks = false;
    let mut seed: Option<u64> = None;
    let mut tracked_pointer_tag: Option<miri::PtrId> = None;
    let mut tracked_alloc_id: Option<miri::AllocId> = None;
    let mut rustc_args = vec![];
    let mut crate_args = vec![];
    let mut after_dashdash = false;
    let mut excluded_env_vars = vec![];
    for arg in env::args() {
        if rustc_args.is_empty() {
            // Very first arg: binary name.
            rustc_args.push(arg);
        } else if after_dashdash {
            // Everything that comes after `--` is forwarded to the interpreted crate.
            crate_args.push(arg);
        } else {
            match arg.as_str() {
                "-Zmiri-disable-validation" => {
                    validate = false;
                }
                "-Zmiri-disable-stacked-borrows" => {
                    stacked_borrows = false;
                }
                "-Zmiri-disable-alignment-check" => {
                    check_alignment = false;
                }
                "-Zmiri-disable-isolation" => {
                    communicate = true;
                }
                "-Zmiri-ignore-leaks" => {
                    ignore_leaks = true;
                }
                "--" => {
                    after_dashdash = true;
                }
                arg if arg.starts_with("-Zmiri-seed=") => {
                    if seed.is_some() {
                        panic!("Cannot specify -Zmiri-seed multiple times!");
                    }
                    let seed_raw = hex::decode(arg.trim_start_matches("-Zmiri-seed="))
                        .unwrap_or_else(|err| match err {
                            FromHexError::InvalidHexCharacter { .. } => panic!(
                                "-Zmiri-seed should only contain valid hex digits [0-9a-fA-F]"
                            ),
                            FromHexError::OddLength =>
                                panic!("-Zmiri-seed should have an even number of digits"),
                            err => panic!("Unknown error decoding -Zmiri-seed as hex: {:?}", err),
                        });
                    if seed_raw.len() > 8 {
                        panic!(format!(
                            "-Zmiri-seed must be at most 8 bytes, was {}",
                            seed_raw.len()
                        ));
                    }

                    let mut bytes = [0; 8];
                    bytes[..seed_raw.len()].copy_from_slice(&seed_raw);
                    seed = Some(u64::from_be_bytes(bytes));
                }
                arg if arg.starts_with("-Zmiri-env-exclude=") => {
                    excluded_env_vars
                        .push(arg.trim_start_matches("-Zmiri-env-exclude=").to_owned());
                }
                arg if arg.starts_with("-Zmiri-track-pointer-tag=") => {
                    let id: u64 = match arg.trim_start_matches("-Zmiri-track-pointer-tag=").parse()
                    {
                        Ok(id) => id,
                        Err(err) => panic!(
                            "-Zmiri-track-pointer-tag requires a valid `u64` as the argument: {}",
                            err
                        ),
                    };
                    if let Some(id) = miri::PtrId::new(id) {
                        tracked_pointer_tag = Some(id);
                    } else {
                        panic!("-Zmiri-track-pointer-tag must be a nonzero id");
                    }
                }
                arg if arg.starts_with("-Zmiri-track-alloc-id=") => {
                    let id: u64 = match arg.trim_start_matches("-Zmiri-track-alloc-id=").parse()
                    {
                        Ok(id) => id,
                        Err(err) => panic!(
                            "-Zmiri-track-alloc-id requires a valid `u64` as the argument: {}",
                            err
                        ),
                    };
                    tracked_alloc_id = Some(miri::AllocId(id));
                }
                _ => {
                    // Forward to rustc.
                    rustc_args.push(arg);
                }
            }
        }
    }

    debug!("rustc arguments: {:?}", rustc_args);
    debug!("crate arguments: {:?}", crate_args);
    let miri_config = miri::MiriConfig {
        validate,
        stacked_borrows,
        check_alignment,
        communicate,
        ignore_leaks,
        excluded_env_vars,
        seed,
        args: crate_args,
        tracked_pointer_tag,
        tracked_alloc_id,
    };
    run_compiler(rustc_args, &mut MiriCompilerCalls { miri_config })
}
