use starling::aabb::AABB;
use starling::prelude::Vec2;
use svg::node::element::path::Data;
use svg::node::element::Path;
use svg::Document;

pub fn write_svg(filepath: &str, aabbs: &[AABB]) -> Result<(), std::io::Error> {
    let padding = 10.0;

    if aabbs.is_empty() {
        return Ok(());
    }

    let mut bounds = aabbs[0];

    for aabb in aabbs {
        bounds.include(&aabb.lower());
        bounds.include(&aabb.upper());
    }

    let l = bounds.lower() - Vec2::splat(padding);
    let u = bounds.upper() + Vec2::splat(padding);
    let w = u - l;

    let mut doc = Document::new().set("viewBox", (l.x, l.y, w.x, w.y));

    for (i, aabb) in (&[bounds]).iter().chain(aabbs).enumerate() {
        let corners = aabb.corners();

        println!("{:?}", aabb);

        let to_tup = |p: Vec2| (p.x, p.y);

        let data = Data::new()
            .move_to(to_tup(corners[0]))
            .line_to(to_tup(corners[1]))
            .line_to(to_tup(corners[2]))
            .line_to(to_tup(corners[3]))
            .line_to(to_tup(corners[0]))
            .close();

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", if i == 0 { "grey" } else { "red" })
            .set("stroke-width", 1)
            .set("d", data);

        doc = doc.add(path);
    }

    svg::save(filepath, &doc)
}

#[cfg(test)]
mod tests {

    use super::*;
    use starling::math::rand;

    #[test]
    fn svg_test() {
        let aabbs = (0..20)
            .map(|_| {
                let p1 = (rand(50.0, 600.0), rand(50.0, 600.0));
                let p2 = (rand(50.0, 600.0), rand(50.0, 600.0));
                AABB::from_arbitrary(p1, p2)
            })
            .collect::<Vec<_>>();

        write_svg("boxes.svg", &aabbs).unwrap();
    }

    #[test]
    fn svg_layout() {
        let mut dims = vec![
            [(0.0, 0.0), (0.1, 1.0)],
            [(0.1, 0.0), (1.0, 0.1)],
            [(0.8, 0.95), (1.0, 1.0)],
            // window
            [(0.4, 0.5), (0.6, 0.8)],
            [(0.41, 0.79), (0.59, 0.75)],
        ];

        for i in 0..20 {
            let w = 1.0 / 21.0;
            let s = i as f32 * w + 0.03;
            dims.push([(0.01, s), (0.09, s + w - 0.003)])
        }

        let w = 700.0;
        let h = 500.0;

        let aabbs: Vec<AABB> = dims
            .iter()
            .map(|x| {
                let p: Vec2 = x[0].into();
                let p = p.with_y(1.0 - p.y);
                let q: Vec2 = x[1].into();
                let q = q.with_y(1.0 - q.y);
                let s = Vec2::new(w, h);
                AABB::from_arbitrary(p * s, q * s)
            })
            .collect();

        write_svg("layout.svg", &aabbs).unwrap();
    }
}
