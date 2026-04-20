// msvcrt demo — calling C runtime functions via dyncall + Boa
// Run: cargo run --bin demo -- msvcrt.js

// ── 1. Simple integer function ────────────────────────────────────────────────
const abs = exfun("msvcrt.dll|abs|i32|i32|");
console.log("abs(-42)      =", abs(-42));          // 42

// ── 2. C string argument ──────────────────────────────────────────────────────
const atoi  = exfun("msvcrt.dll|atoi|cstr|i32|");
const stlen = exfun("msvcrt.dll|strlen|cstr|u64|");
console.log('atoi("123")   =', atoi("123"));       // 123
console.log('strlen("hi")  =', stlen("hi"));       // 2

// ── 3. C string return value ──────────────────────────────────────────────────
const strerror = exfun("msvcrt.dll|strerror|i32|cstr|");
console.log("strerror(2)   =", strerror(2));       // "No such file or directory"

// ── 4. Multiple arguments ─────────────────────────────────────────────────────
const pow = exfun("msvcrt.dll|pow|f64,f64|f64|");
console.log("pow(2, 10)    =", pow(2, 10));        // 1024

// ── 5. JS wrapper around a native function ────────────────────────────────────
const _abs = exfun("msvcrt.dll|abs|i32|i32|");
function safeAbs(n) {
    if (typeof n !== "number") throw new TypeError("expected number");
    return _abs(n);
}
console.log("safeAbs(-7)   =", safeAbs(-7));       // 7

// ── 6. Same descriptor registered twice ──────────────────────────────────────
const abs2 = exfun("msvcrt.dll|abs|i32|i32|");
console.log("abs2(-100)    =", abs2(-100));        // 100
