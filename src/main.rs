//! Generic Boa JS runner with dyncall FFI support.
//!
//! Usage:
//!   cargo run --bin demo -- <script.js>
//!
//! Registers: console, exfun (dyncall), checkQuit (SDL event polling)
//! Adds C:\tools to the DLL search path so SDL2_image can find SDL2.dll.

use std::cell::RefCell;

use boa_engine::{js_string, native_function::NativeFunction, property::Attribute, Context, JsValue, Source};
use boa_runtime::Console;
use boa_dyncall::register_dyncall;
use dyncall::{DynCaller, ScriptVal};

/// Add a directory to Windows' DLL search path for this process.
fn add_dll_directory(dir: &str) {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetDllDirectoryA(path: *const u8) -> i32;
    }
    let mut buf: Vec<u8> = dir.bytes().collect();
    buf.push(0);
    unsafe { SetDllDirectoryA(buf.as_ptr()); }
}

/// Register a `checkQuit()` JS function that polls SDL_PollEvent.
/// Returns `true` if an SDL_QUIT event (type == 0x100) is pending.
fn register_check_quit(ctx: &mut Context) {
    const SDL_QUIT_TYPE: u32 = 0x100;

    thread_local! {
        static EVENT_BUF: RefCell<[u8; 56]> = const { RefCell::new([0u8; 56]) };
        static POLL_FN: RefCell<Option<dyncall::FuncDef>> = const { RefCell::new(None) };
    }

    ctx.register_global_builtin_callable(
        js_string!("checkQuit"),
        0,
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
            POLL_FN.with(|cell| {
                let mut slot = cell.borrow_mut();
                if slot.is_none() {
                    *slot = Some(
                        DynCaller::define_function(
                            r"C:\tools\SDL2.dll|SDL_PollEvent|ptr|i32|",
                        )
                        .expect("SDL_PollEvent not found in SDL2.dll"),
                    );
                }
            });

            let buf_ptr: *mut std::ffi::c_void =
                EVENT_BUF.with(|buf| buf.borrow_mut().as_mut_ptr() as *mut _);

            let mut quit = false;

            POLL_FN.with(|cell| {
                let fdef_ref = cell.borrow();
                let fdef = fdef_ref.as_ref().unwrap();
                loop {
                    let mut inv = fdef.prep();
                    inv.push_script_val(ScriptVal::Pointer(buf_ptr))
                        .expect("push event buf ptr");
                    match inv.call().expect("SDL_PollEvent") {
                        dyncall::ArgVal::I32(0) => break,
                        _ => {
                            let event_type = EVENT_BUF.with(|buf| {
                                let b = buf.borrow();
                                u32::from_ne_bytes([b[0], b[1], b[2], b[3]])
                            });
                            if event_type == SDL_QUIT_TYPE {
                                quit = true;
                            }
                        }
                    }
                }
            });

            Ok(JsValue::from(quit))
        }),
    )
    .expect("checkQuit registration failed");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script.js>", args[0]);
        std::process::exit(1);
    }
    let script_path = &args[1];
    let source = std::fs::read_to_string(script_path)
        .unwrap_or_else(|e| { eprintln!("Cannot read {script_path}: {e}"); std::process::exit(1) });

    add_dll_directory(r"C:\tools");

    let mut ctx = Context::default();

    let console = Console::init(&mut ctx);
    ctx.register_global_property(Console::NAME, console, Attribute::all())
        .expect("console install failed");

    register_dyncall(&mut ctx);
    register_check_quit(&mut ctx);

    // Inject the working directory as __dir so scripts can build absolute paths.
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.register_global_property(
        js_string!("__dir"),
        JsValue::from(js_string!(cwd.as_str())),
        Attribute::all(),
    ).expect("__dir injection failed");

    match ctx.eval(Source::from_bytes(source.as_bytes())) {
        Ok(result) => {
            if !result.is_undefined() {
                println!("=> {}", result.display());
            }
        }
        Err(e) => eprintln!("JS error: {e}"),
    }
}
