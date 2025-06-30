use crate::geometry::aabb::Aabb;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::{format_point, format_scalar};
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use cgmath::{AbsDiffEq, Array, ElementWise, EuclideanSpace, InnerSpace, MetricSpace};
use std::rc::Rc;

pub struct SdfVesicaSegment {
    width: f64,
    start: Point,
    end: Point,
}

impl SdfVesicaSegment {
    #[must_use]
    pub fn new(width: f64, start: Point, end: Point) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        assert!(width <= start.distance(end), "width cannot be greater than segment length");
        Rc::new(Self { width, start, end, })
    }
}

impl Sdf for SdfVesicaSegment {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let a = {start};\n\
            let b = {end};\n\
            let w = {width};\n\
            let c = (a+b)*0.5;\n\
            let l = length(b-a);\n\
            let v = (b-a)/l;\n\
            let y = dot({parameter}-c, v);\n\
            let q = vec2f(length({parameter}-c-y*v), abs(y));\n\
            let r = 0.5*l;\n\
            let d = 0.5*(r*r-w*w)/w;\n\
            var h: vec3f; \
            if (r*q.x<d*(q.y-r)) {{ h = vec3f(0.0,r,0.0); }} else {{ h = vec3f(-d,0.0,d+w); }}\n\
            return length(q-h.xy) - h.z;",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            start = format_point(self.start),
            end = format_point(self.end),
            width = format_scalar(self.width),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        /*
        The vesica's AABB is equals to cylinder's with the
        flat caps and same point.

        https://gdalgorithms-list.narkive.com/s2wbl3Cd/algorithms-axis-aligned-bounding-box-of-cylinder

        This amounts to projecting the cylinder to all three coordinate axes.
        
        To project a cylinder to an axis in general, you project both the
        center line (with its length) and a side (circle) to each axis. The
        sum of those two lengths (which is in effect the length of the
        Minowski sum of the line and the circle) will be the extent along that
        axis. The extent is, of course, centered around the cylinder's center.
        
        The formula should be something like (IIRC): d*l + 2*r*sqrt(1-d*d)
        where d is dot product of the center line and the axis, l is length
        and r is the radius.
        */
        let direction = self.end - self.start;
        let length = direction.magnitude();
        
        if length.abs_diff_eq(&0.0, DEFAULT_EPSILON_F64) {
            return Aabb::make_minimal();
        }
        
        let axis = direction / length;
        let extent = 
            abs_element_wise(direction) + 
            (2.0 * self.width * sqrt_element_wise(Vector::from_value(1.0) - axis.mul_element_wise(axis)));
        let half_extent = extent / 2.0;
        let center = (self.start.to_vec() + self.end.to_vec()) * 0.5;
        
        return Aabb::from_points(Point::from_vec(center - half_extent), Point::from_vec(center + half_extent));

        #[must_use]
        fn sqrt_element_wise(this: Vector) -> Vector {
            Vector::new(this.x.sqrt(), this.y.sqrt(), this.z.sqrt())
        }
        #[must_use]
        fn abs_element_wise(this: Vector) -> Vector {
            Vector::new(this.x.abs(), this.y.abs(), this.z.abs())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let width = 1.0;
        let start = Point::new(0.0, -1.0, 0.0);
        let end = Point::new(0.0, 1.0, 0.0);
        let system_under_test = SdfVesicaSegment::new(width, start, end);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let width = 0.5;
        let start = Point::new(-1.0, -2.0, -3.0);
        let end = Point::new(4.0, 5.0, 6.0);
        let system_under_test = SdfVesicaSegment::new(width, start, end);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let a = vec3f(-1.0,-2.0,-3.0);\nlet b = vec3f(4.0,5.0,6.0);\nlet w = 0.5;\nlet c = (a+b)*0.5;\nlet l = length(b-a);\nlet v = (b-a)/l;\nlet y = dot(point-c, v);\nlet q = vec2f(length(point-c-y*v), abs(y));\nlet r = 0.5*l;\nlet d = 0.5*(r*r-w*w)/w;\nvar h: vec3f; if (r*q.x<d*(q.y-r)) { h = vec3f(0.0,r,0.0); } else { h = vec3f(-d,0.0,d+w); }\nreturn length(q-h.xy) - h.z;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}