//! dyncall integration for the [Boa](https://boajs.dev/) JavaScript engine.
//!
//! Call [`register_dyncall`] once on a [`Context`] to install all bindings.
//!
//! # JavaScript API
//!
//! | JS expression | What it does |
//! |---|---|
//! | `exfun(descriptor)` | Parse descriptor, return a callable JS function |
//! | `new ExStruct(descriptor)` | Allocate a zeroed C struct whose layout is taken from the first struct-typed arg (or return type) in the descriptor |
//! | `s.getField(i)` | Read field `i` as a JS number or string |
//! | `s.setField(i, v)` | Write field `i` |
//! | `s.fieldCount()` | Number of fields |
//!
//! # Example
//!
//! ```ignore
//! use boa_engine::{Context, Source};
//! use boa_dyncall::register_dyncall;
//!
//! let mut ctx = Context::default();
//! register_dyncall(&mut ctx);
//!
//! ctx.eval(Source::from_bytes(r#"
//!     const abs = exfun("msvcrt.dll|abs|i32|i32|");
//!     console.log(abs(-42));   // 42
//! "#)).unwrap();
//! ```

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use boa_engine::{
    class::{Class, ClassBuilder},
    js_string,
    native_function::NativeFunction,
    Context, JsArgs, JsData, JsError, JsNativeError, JsResult, JsString, JsValue,
};
use boa_gc::{Finalize, Trace};
use dyncall::{ArgType, DynCaller, FuncDef, ScriptVal, StructValue};

// ── FuncDefCapture ─────────────────────────────────────────────────────────────
// Wraps FuncDef to satisfy the `Trace + Clone + 'static` bound required by
// NativeFunction::from_closure_with_captures.  FuncDef contains no GC-managed
// objects, so declaring it as non-traceable is safe.
#[derive(Clone, Trace, Finalize)]
struct FuncDefCapture(#[unsafe_ignore_trace] FuncDef);

// ── PENDING thread-local ───────────────────────────────────────────────────────
// Allows Rust code to inject a pre-built StructValue into ExStruct::data_constructor
// without needing to round-trip through the descriptor parser.
thread_local! {
    static PENDING: RefCell<Option<(StructValue, ArgType)>> = const { RefCell::new(None) };
}

// ── ExStruct ──────────────────────────────────────────────────────────────────

/// A C struct value exposed to JavaScript.
///
/// Create with `new ExStruct("lib|func|*{i32,f64}|void|")` (or with the
/// `exstruct` convenience function which does the same thing).
///
/// Uses `Arc<Mutex<StructValue>>` so that when a `*{…}` call mutates the struct
/// through a pointer, all JavaScript variables referencing the same instance see
/// the updated field values.
#[derive(Trace, Finalize, JsData)]
pub struct ExStruct {
    #[unsafe_ignore_trace]
    sv: Arc<Mutex<StructValue>>,
    #[unsafe_ignore_trace]
    #[allow(dead_code)]
    arg_type: ArgType,
}

// ── Error helper ───────────────────────────────────────────────────────────────

fn js_err(msg: impl Into<String>) -> JsError {
    JsNativeError::error().with_message(msg.into()).into()
}

// ── ExStruct Class impl ────────────────────────────────────────────────────────

impl Class for ExStruct {
    const NAME: &'static str = "ExStruct";
    const LENGTH: usize = 1;

    fn data_constructor(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<Self> {
        // Fast path: Rust code pre-staged a StructValue (e.g. for struct return values).
        if let Some((sv, at)) = PENDING.with(|c| c.borrow_mut().take()) {
            return Ok(ExStruct {
                sv: Arc::new(Mutex::new(sv)),
                arg_type: at,
            });
        }

        // Normal JS path: new ExStruct("lib|func|*{i32,f64}|void|")
        let descriptor = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        let (sv, at) = struct_from_descriptor(&descriptor)?;
        Ok(ExStruct {
            sv: Arc::new(Mutex::new(sv)),
            arg_type: at,
        })
    }

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        // ── getField(i) ───────────────────────────────────────────────────────
        class.method(
            js_string!("getField"),
            1,
            NativeFunction::from_fn_ptr(|this, args, context| {
                let idx = args.get_or_undefined(0).to_index(context)? as usize;
                with_sv(this, |sv| {
                    match sv
                        .script_read(idx)
                        .map_err(|e| js_err(format!("getField: {e}")))?
                    {
                        ScriptVal::Number(n) => Ok(JsValue::from(n)),
                        ScriptVal::Integer(n) => Ok(JsValue::from(n as f64)),
                        ScriptVal::Str(s) => Ok(JsValue::from(JsString::from(s.as_str()))),
                        ScriptVal::Pointer(p) => Ok(JsValue::from(p as usize as f64)),
                        ScriptVal::Nil => Ok(JsValue::null()),
                    }
                })
            }),
        );

        // ── setField(i, v) ────────────────────────────────────────────────────
        class.method(
            js_string!("setField"),
            2,
            NativeFunction::from_fn_ptr(|this, args, context| {
                let idx = args.get_or_undefined(0).to_index(context)? as usize;
                let val = args.get_or_undefined(1);
                with_sv_mut(this, |sv| {
                    let fc = sv.field_count();
                    if idx >= fc {
                        return Err(js_err(format!(
                            "setField: index {idx} out of bounds ({fc})"
                        )));
                    }
                    // Read all fields, replace at idx, rebuild.
                    let mut fields: Vec<ScriptVal> = (0..fc)
                        .map(|fi| {
                            sv.script_read(fi)
                                .map_err(|e| js_err(format!("setField read {fi}: {e}")))
                        })
                        .collect::<Result<_, _>>()?;

                    fields[idx] = js_val_to_script_val(&val);

                    sv.reset();
                    for f in &fields {
                        let r = match f {
                            ScriptVal::Number(n) => sv.push_field_coerced(n),
                            ScriptVal::Integer(n) => sv.push_field_coerced(n),
                            ScriptVal::Str(_) => sv.push_field_coerced(&0.0f64),
                            ScriptVal::Pointer(p) => sv.push_field(p),
                            ScriptVal::Nil => sv.push_field_coerced(&0i64),
                        };
                        r.map_err(|e| js_err(format!("setField rebuild: {e}")))?;
                    }
                    Ok(JsValue::undefined())
                })
            }),
        );

        // ── fieldCount() ──────────────────────────────────────────────────────
        class.method(
            js_string!("fieldCount"),
            0,
            NativeFunction::from_fn_ptr(|this, _args, _context| {
                with_sv(this, |sv| Ok(JsValue::from(sv.field_count() as f64)))
            }),
        );

        Ok(())
    }
}

// ── ExStruct lock helpers ──────────────────────────────────────────────────────

fn with_sv<F, R>(this: &JsValue, f: F) -> JsResult<R>
where
    F: FnOnce(&StructValue) -> JsResult<R>,
{
    let obj = this.as_object().ok_or_else(|| js_err("not an ExStruct"))?;
    let ex = obj
        .downcast_ref::<ExStruct>()
        .ok_or_else(|| js_err("not an ExStruct"))?;
    let sv = ex.sv.lock().map_err(|_| js_err("mutex poisoned"))?;
    f(&sv)
}

fn with_sv_mut<F, R>(this: &JsValue, f: F) -> JsResult<R>
where
    F: FnOnce(&mut StructValue) -> JsResult<R>,
{
    let obj = this.as_object().ok_or_else(|| js_err("not an ExStruct"))?;
    let ex = obj
        .downcast_ref::<ExStruct>()
        .ok_or_else(|| js_err("not an ExStruct"))?;
    let mut sv = ex.sv.lock().map_err(|_| js_err("mutex poisoned"))?;
    f(&mut sv)
}

// ── Value conversion ───────────────────────────────────────────────────────────

fn js_val_to_script_val(val: &JsValue) -> ScriptVal {
    if val.is_null() || val.is_undefined() {
        ScriptVal::Nil
    } else if let Some(n) = val.as_number() {
        ScriptVal::Number(n)
    } else if let Some(s) = val.as_string() {
        ScriptVal::Str(s.to_std_string_escaped())
    } else {
        ScriptVal::Nil
    }
}

fn script_val_to_jsvalue(sv: ScriptVal) -> JsValue {
    match sv {
        ScriptVal::Nil => JsValue::undefined(),
        ScriptVal::Integer(n) => JsValue::from(n as f64),
        ScriptVal::Number(n) => JsValue::from(n),
        ScriptVal::Str(s) => JsValue::from(JsString::from(s.as_str())),
        ScriptVal::Pointer(p) => JsValue::from(p as usize as f64),
    }
}

// ── Descriptor → StructValue ───────────────────────────────────────────────────

fn struct_from_descriptor(descriptor: &str) -> JsResult<(StructValue, ArgType)> {
    let fdef = DynCaller::define_function(descriptor)
        .map_err(|e| js_err(format!("invalid descriptor: {e}")))?;

    let arg_type = (0..fdef.get_arg_count())
        .map(|i| fdef.get_arg_type(i))
        .find(|at| at.struct_type().is_some())
        .ok_or_else(|| js_err("no struct type found in descriptor"))?
        .clone();

    let sv = StructValue::new(&arg_type).map_err(|e| js_err(format!("{e}")))?;
    Ok((sv, arg_type))
}

// ── dispatch ───────────────────────────────────────────────────────────────────

fn dispatch(fdef: &FuncDef, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let expected = fdef.get_arg_count();
    if args.len() != expected {
        return Err(js_err(format!(
            "expected {expected} args, got {}",
            args.len()
        )));
    }

    // Clone the Arc for each struct argument so we can lock them independently
    // and keep the locks alive across the entire ffi_call.
    let struct_arcs: Vec<Option<Arc<Mutex<StructValue>>>> = (0..args.len())
        .map(|i| {
            let at = fdef.get_arg_type(i);
            if at.struct_type().is_some() {
                if let Some(obj) = args[i].as_object() {
                    obj.downcast_ref::<ExStruct>().map(|es| es.sv.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut struct_guards: Vec<Option<std::sync::MutexGuard<'_, StructValue>>> = struct_arcs
        .iter()
        .map(|opt| opt.as_ref().map(|a| a.lock().unwrap()))
        .collect();

    let mut inv = fdef.prep();

    for i in 0..args.len() {
        let at = fdef.get_arg_type(i).clone();
        match &at {
            // ── struct by value ───────────────────────────────────────────────
            ArgType::Struct(_) => {
                let guard = struct_guards[i]
                    .as_ref()
                    .ok_or_else(|| js_err(format!("arg {i}: expected ExStruct")))?;
                inv.push_arg(&**guard)
                    .map_err(|e| js_err(format!("arg {i}: {e}")))?;
            }

            // ── struct by pointer (write-back) ────────────────────────────────
            ArgType::Pointer(inner) if matches!(inner.as_ref(), ArgType::Struct(_)) => {
                let guard = struct_guards[i]
                    .as_mut()
                    .ok_or_else(|| js_err(format!("arg {i}: expected ExStruct")))?;
                inv.push_mut_arg(&mut **guard)
                    .map_err(|e| js_err(format!("arg {i}: {e}")))?;
            }

            // ── everything else: push_script_val handles cstr, ptr, scalars ──
            _ => {
                let sv = js_val_to_script_val(&args[i]);
                inv.push_script_val(sv)
                    .map_err(|e| js_err(format!("arg {i}: {e}")))?;
            }
        }
    }

    drop(struct_guards);
    let result = inv.call_scripted().map_err(|e| js_err(format!("call failed: {e}")))?;
    Ok(script_val_to_jsvalue(result.return_val))
}

// ── public API ─────────────────────────────────────────────────────────────────

/// Register the dyncall integration into `context`.
///
/// Installs:
/// - **`exfun(descriptor)`** → JS function — parses the descriptor and returns
///   a native JS function that calls the described C function.
/// - **`ExStruct`** — global class for C struct values, with `getField`,
///   `setField`, and `fieldCount` methods.
///
/// Must be called before any scripts that use `exfun` or `ExStruct`.
pub fn register_dyncall(context: &mut Context) {
    context
        .register_global_class::<ExStruct>()
        .expect("ExStruct class registration failed");

    context
        .register_global_builtin_callable(
            js_string!("exfun"),
            1,
            NativeFunction::from_fn_ptr(|_this, args, context| {
                let descriptor = args
                    .get_or_undefined(0)
                    .to_string(context)?
                    .to_std_string_escaped();

                let fdef = DynCaller::define_function(&descriptor)
                    .map_err(|e| js_err(format!("exfun: {e}")))?;

                let capture = FuncDefCapture(fdef);
                let func = unsafe {
                    NativeFunction::from_closure_with_captures(
                        |_this, args, cap: &FuncDefCapture, context| {
                            dispatch(&cap.0, args, context)
                        },
                        capture,
                    )
                };

                Ok(JsValue::from(func.to_js_function(context.realm())))
            }),
        )
        .expect("exfun registration failed");
}
