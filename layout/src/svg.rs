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

    for aabb in aabbs {
        let corners = aabb.corners();

        let to_tup = |p: Vec2| (p.x, p.y);

        let data = Data::new()
            .move_to(to_tup(corners[0]))
            .line_to(to_tup(corners[1]))
            .line_to(to_tup(corners[2]))
            .line_to(to_tup(corners[3]))
            .line_to(to_tup(corners[0]))
            .close();

        let path = Path::new()
            .set("fill", "white")
            .set("fill-opacity", 1)
            .set("stroke", "blue")
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
}
