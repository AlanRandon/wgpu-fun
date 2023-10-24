#[cfg(test)]
use cgmath::vec2;
use cgmath::{InnerSpace, Vector2};

/// checks if p is to the right of line ab
fn is_right(p: Vector2<f32>, a: Vector2<f32>, b: Vector2<f32>) -> bool {
    // line is vertical
    if a.x == b.x {
        return p.x > a.x;
    }

    // line is horizontal
    if a.y == b.y {
        return p.y > a.y;
    }

    // solve y = mx + c for line ab
    let (dx, dy) = b.zip(a, std::ops::Sub::sub).into();
    let m = dy / dx;
    // c = y - mx
    let c = a.y - m * a.x;

    let ab_y = m * p.x + c;
    p.y > ab_y
}

pub fn triangle_contains(
    p: Vector2<f32>,
    v1: Vector2<f32>,
    v2: Vector2<f32>,
    v3: Vector2<f32>,
) -> bool {
    let s1 = is_right(p, v1, v2) ^ is_right(v3, v1, v2);
    let s2 = is_right(p, v1, v3) ^ is_right(v2, v1, v3);
    let s3 = is_right(p, v2, v3) ^ is_right(v1, v2, v3);

    !s1 & !s2 & !s3
}

#[test]
fn triangle_contains_works() {
    assert!(triangle_contains(
        vec2(0.1, 0.1),
        vec2(0., 0.),
        vec2(1., 0.),
        vec2(0., 1.)
    ));

    assert!(!triangle_contains(
        vec2(1., 1.),
        vec2(0., 0.),
        vec2(1., 0.),
        vec2(0., 1.)
    ));

    assert!(!triangle_contains(
        vec2(-0.1, -0.1),
        vec2(0., 0.),
        vec2(1., 0.),
        vec2(0., 1.)
    ));
}

pub fn circle_intersects_line_segment(
    c: Vector2<f32>,
    r: f32,
    a: Vector2<f32>,
    b: Vector2<f32>,
) -> bool {
    let closest_point = {
        let line = b - a;
        let line_norm = line.normalize();
        let ac = c - a;
        let t = ac.dot(line_norm);
        if t < 0.0 {
            a
        } else if t > line.magnitude() {
            b
        } else {
            a + line_norm * t
        }
    };

    let distance = (c - closest_point).magnitude();

    distance <= r
}

#[test]
fn circle_intersects_line_segment_works() {
    assert!(circle_intersects_line_segment(
        vec2(0., 0.),
        1.,
        vec2(-1., -1.),
        vec2(1., 1.)
    ));

    assert!(circle_intersects_line_segment(
        vec2(0., 0.),
        1.,
        vec2(0., 0.),
        vec2(1., 1.)
    ));

    assert!(circle_intersects_line_segment(
        vec2(0., 0.),
        1.,
        vec2(0., 0.),
        vec2(0.1, 0.1)
    ));

    assert!(!circle_intersects_line_segment(
        vec2(0., 0.),
        1.,
        vec2(0., 2.),
        vec2(0., 2.)
    ));
}

pub fn circle_intersects_triangle(
    c: Vector2<f32>,
    r: f32,
    v1: Vector2<f32>,
    v2: Vector2<f32>,
    v3: Vector2<f32>,
) -> bool {
    triangle_contains(c, v1, v2, v3)
        | circle_intersects_line_segment(c, r, v1, v2)
        | circle_intersects_line_segment(c, r, v1, v3)
        | circle_intersects_line_segment(c, r, v2, v3)
}
