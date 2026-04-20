// Struct fixture for the dyncall struct demo.
// Compiled to a cdylib by build.rs — do not modify without updating struct_demo.js.

#[repr(C)]
pub struct Point {
    x: f64,
    y: f64,
}

/// Takes a Point by value; returns the squared distance from the origin.
#[no_mangle]
pub extern "C" fn point_dist_sq(p: Point) -> f64 {
    p.x * p.x + p.y * p.y
}

/// Takes a Point by const pointer; returns the Manhattan distance from the origin.
#[no_mangle]
pub extern "C" fn point_manhattan(p: *const Point) -> f64 {
    unsafe { (*p).x.abs() + (*p).y.abs() }
}

/// Takes a Point by mutable pointer; scales both fields by `factor` in-place.
/// Returns the new squared distance.
#[no_mangle]
pub extern "C" fn point_scale(p: *mut Point, factor: f64) -> f64 {
    unsafe {
        (*p).x *= factor;
        (*p).y *= factor;
        (*p).x * (*p).x + (*p).y * (*p).y
    }
}
