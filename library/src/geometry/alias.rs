use cgmath::Point3;
use cgmath::Vector3;

pub type Point = Point3<f64>;
pub type Vector = Vector3<f64>;

#[must_use]
pub(crate) fn format_point(point: Point) -> String {
    const MAX_CHARS_TO_OUTPUT: usize = 5;
    let format_coord = |coord: f64| -> String {
        let s = format!("{:.3}", coord);
        if s.len() <= MAX_CHARS_TO_OUTPUT {
            s
        } else {
            s.chars().take(MAX_CHARS_TO_OUTPUT).collect()
        }
    };
    format!("{},{},{}", format_coord(point.x), format_coord(point.y), format_coord(point.z))
}
