// Struct args demo — dyncall + Boa
//
// Run: cargo run --bin demo -- struct_demo.js
//
// The struct_fixture.dll is built automatically by build.rs during `cargo build`.
// It exports:
//   point_dist_sq  ({f64,f64}) -> f64     struct by value
//   point_manhattan(*{f64,f64}) -> f64    struct by const pointer
//   point_scale    (*{f64,f64}, f64) -> f64   struct by mutable pointer (write-back)

const DLL = `${__dir}\\target\\struct-fixture\\struct_fixture.dll`;

// ── Bind the three fixture functions ─────────────────────────────────────────
const dist_sq   = exfun(`${DLL}|point_dist_sq|{f64,f64}|f64|`);
const manhattan = exfun(`${DLL}|point_manhattan|*{f64,f64}|f64|`);
const scale     = exfun(`${DLL}|point_scale|*{f64,f64},f64|f64|`);

// ── Helper: print all fields of an ExStruct ───────────────────────────────────
function printPoint(label, s) {
    console.log(`${label}: x=${s.getField(0)}, y=${s.getField(1)}`);
}

// ─────────────────────────────────────────────────────────────────────────────
// Demo 1: struct by value
//   The struct is copied into the function; the original is unchanged.
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== Demo 1: struct passed by value ===");

const p1 = new ExStruct(`${DLL}|point_dist_sq|{f64,f64}|f64|`);
p1.setField(0, 3.0);   // x
p1.setField(1, 4.0);   // y
printPoint("p1 before", p1);

const distSq = dist_sq(p1);
console.log("point_dist_sq(3, 4) =", distSq);      // 25
console.log("Expected: 25");

printPoint("p1 after (unchanged)", p1);
console.log();

// ─────────────────────────────────────────────────────────────────────────────
// Demo 2: struct by const pointer (read-only)
//   The function reads the struct but doesn't modify it.
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== Demo 2: struct by const pointer (read-only) ===");

const p2 = new ExStruct(`${DLL}|point_manhattan|*{f64,f64}|f64|`);
p2.setField(0, -5.0);  // x
p2.setField(1,  3.0);  // y
printPoint("p2 before", p2);

const manDist = manhattan(p2);
console.log("point_manhattan(-5, 3) =", manDist);  // 8
console.log("Expected: 8");

printPoint("p2 after (unchanged)", p2);
console.log();

// ─────────────────────────────────────────────────────────────────────────────
// Demo 3: struct by mutable pointer (write-back)
//   point_scale multiplies both x and y by factor in-place.
//   After the call the JS ExStruct reflects the updated values.
// ─────────────────────────────────────────────────────────────────────────────
console.log("=== Demo 3: struct by mutable pointer (write-back) ===");

const p3 = new ExStruct(`${DLL}|point_scale|*{f64,f64},f64|f64|`);
p3.setField(0, 2.0);   // x
p3.setField(1, 3.0);   // y
printPoint("p3 before scale(×10)", p3);

const newDistSq = scale(p3, 10.0);
console.log("point_scale(p3, 10) returned distSq =", newDistSq);  // 1300
console.log("Expected: 1300");

printPoint("p3 after (mutated by C code)", p3);     // x=20, y=30
console.log("Expected: x=20, y=30");
