// Windows API struct demo — dyncall + Boa
//
// Calls real kernel32/user32 functions that take struct pointer arguments.
// Run: cargo run --bin demo -- winapi_structs.js

// ─────────────────────────────────────────────────────────────────────────────
// Demo 1: GetLocalTime — SYSTEMTIME (8 × u16 fields, void return)
//
//   typedef struct _SYSTEMTIME {
//     WORD wYear, wMonth, wDayOfWeek, wDay,
//          wHour, wMinute, wSecond, wMilliseconds;
//   } SYSTEMTIME;
//
//   void GetLocalTime(LPSYSTEMTIME lpSystemTime);
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== GetLocalTime (SYSTEMTIME: 8 × u16) ===");

const GetLocalTime = exfun(
    "kernel32.dll|GetLocalTime|*{u16,u16,u16,u16,u16,u16,u16,u16}|void|"
);

const st = new ExStruct(
    "kernel32.dll|GetLocalTime|*{u16,u16,u16,u16,u16,u16,u16,u16}|void|"
);

GetLocalTime(st);

const days = ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"];
const year  = st.getField(0);
const month = st.getField(1);
const dow   = st.getField(2);
const day   = st.getField(3);
const hour  = st.getField(4);
const min   = st.getField(5);
const sec   = st.getField(6);
const ms    = st.getField(7);

const pad = n => String(n).padStart(2, "0");
console.log(`Date: ${days[dow]}, ${year}-${pad(month)}-${pad(day)}`);
console.log(`Time: ${pad(hour)}:${pad(min)}:${pad(sec)}.${String(ms).padStart(3,"0")}`);
console.log();

// ─────────────────────────────────────────────────────────────────────────────
// Demo 2: GetCursorPos — POINT (2 × i32, returns BOOL)
//
//   typedef struct tagPOINT { LONG x, y; } POINT;
//   BOOL GetCursorPos(LPPOINT lpPoint);
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== GetCursorPos (POINT: 2 × i32) ===");

const GetCursorPos = exfun("user32.dll|GetCursorPos|*{i32,i32}|i32|");

const pt = new ExStruct("user32.dll|GetCursorPos|*{i32,i32}|i32|");
const ok = GetCursorPos(pt);

console.log("GetCursorPos returned:", ok, "(1 = success)");
console.log(`Cursor position: x=${pt.getField(0)}, y=${pt.getField(1)}`);
console.log();

// ─────────────────────────────────────────────────────────────────────────────
// Demo 3: GetWindowRect — RECT (4 × i32, takes HWND + RECT*)
//
//   typedef struct tagRECT { LONG left, top, right, bottom; } RECT;
//   BOOL GetWindowRect(HWND hWnd, LPRECT lpRect);
//   HWND GetForegroundWindow();
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== GetWindowRect (RECT: 4 × i32) ===");

const GetForegroundWindow = exfun("user32.dll|GetForegroundWindow||ptr|");
const GetWindowRect = exfun("user32.dll|GetWindowRect|ptr,*{i32,i32,i32,i32}|i32|");

const hwnd = GetForegroundWindow();
console.log("Foreground HWND:", hwnd);

const rect = new ExStruct("user32.dll|GetWindowRect|ptr,*{i32,i32,i32,i32}|i32|");
const ok2 = GetWindowRect(hwnd, rect);

console.log("GetWindowRect returned:", ok2, "(1 = success)");

const left   = rect.getField(0);
const top    = rect.getField(1);
const right  = rect.getField(2);
const bottom = rect.getField(3);

console.log(`Window rect: left=${left}, top=${top}, right=${right}, bottom=${bottom}`);
console.log(`Window size: ${right - left} × ${bottom - top}`);
