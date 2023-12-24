use egui_macroquad::egui::util::hash;
use egui_macroquad::macroquad::{
    prelude::*,
    rand::{rand, srand},
};
use fast_poisson::Poisson2D;
use geo::line_intersection::line_intersection;
use geo::*;
use undo::*;
use voronator::VoronoiDiagram;

#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    lines: Vec<Line>,
    thickness: f32,
    color: Color,
}

impl Sketch {
    pub fn new(thickness: f32, color: Color) -> Self {
        Sketch {
            lines: Vec::new(),
            thickness,
            color,
        }
    }
    pub fn add(&mut self, line: Line) {
        self.lines.push(line);
    }
    pub fn draw(&self) {
        self.lines
            .iter()
            .for_each(|i| i.draw(self.thickness, self.color))
    }
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line(Vec2, Vec2);

impl Line {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Line(vec2(x1, y1), vec2(x2, y2))
    }
    //should probably implement the From trait instead
    pub fn from_geo(line: geo::Line) -> Self {
        Line(
            vec2(line.start_point().x() as f32, line.start_point().y() as f32),
            vec2(line.end_point().x() as f32, line.end_point().y() as f32),
        )
    }
    pub fn draw(&self, thickness: f32, color: Color) {
        draw_line(self.0.x, self.0.y, self.1.x, self.1.y, thickness, color);
        draw_circle(self.0.x, self.0.y, thickness / 2., color);
        draw_circle(self.1.x, self.1.y, thickness / 2., color);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    area: MultiPolygon,
    sketch: Sketch,
    hatching: Sketch,
}

impl Layer {
    pub fn new() -> Self {
        Layer {
            area: MultiPolygon(vec![]),
            sketch: Sketch::new(3.0, BLACK),
            hatching: Sketch::new(2.0, GRAY),
        }
    }
    pub fn draw(&self) {
        self.hatching.draw();
        self.sketch.draw();
    }
    fn update_sketch(&mut self) {
        self.sketch.clear();
        for l in self.area.lines_iter() {
            self.sketch.add(Line::from_geo(l));
        }
    }
    fn generate_hatching(&mut self) {
        self.hatching.clear();
        let bounding_box = match self.area.bounding_rect() {
            None => return,
            Some(r) => r,
        };
        let radius = 20.;
        let offset = 50.0;
        let hatch_count = 10;
        let points = Poisson2D::new()
            .with_seed(0x5EED)
            .with_dimensions(
                [
                    bounding_box.width() + 2.0 * offset,
                    bounding_box.height() + 2.0 * offset,
                ],
                radius,
            )
            .iter()
            .map(|point| -> (f64, f64) {
                (
                    point[0] + bounding_box.min().x - offset,
                    point[1] + bounding_box.min().y - offset,
                )
            })
            .collect::<Vec<(f64, f64)>>();
        let voronoi: Vec<Polygon> = VoronoiDiagram::<voronator::delaunator::Point>::from_tuple(
            &(bounding_box.min().x - offset, bounding_box.min().y - offset),
            &(bounding_box.max().x + offset, bounding_box.max().y + offset),
            &points,
        )
        .expect("points should give a valid voronoi diagram")
        .cells()
        .iter()
        .map(|polygon| -> Polygon<f64> {
            Polygon::new(
                LineString::from(
                    polygon
                        .points()
                        .iter()
                        .map(|p| (p.x, p.y))
                        .collect::<Vec<_>>(),
                ),
                vec![],
            )
        })
        .filter(|polygon| {
            let centroid = polygon
                .centroid()
                .expect("all polygons should have a centroid");
            !self.area.contains(polygon) && self.area.euclidean_distance(&centroid) <= offset * 0.75
        })
        .map(|polygon| {
            MultiPolygon::from(polygon)
                .boolean_op(&self.area, OpType::Difference)
                .into_iter()
                .next()
                .expect("should be exactly one")
        })
        .collect();
        let hatches_base: Vec<geo::Line> = (-hatch_count..=hatch_count)
            .map(|i| {
                let x = radius * 4. * (i as f64) / (hatch_count as f64);
                geo::Line::new(coord! {x: x, y: -radius * 2.}, coord! {x:x, y: radius * 2.})
            })
            .collect();
        for polygon in voronoi {
            let center = polygon
                .centroid()
                .expect("all polygons should have a centroid");
            srand(hash((center.x() + center.y()).to_bits()));
            let rot = rand() % 360;
            let hatches: Vec<_> = hatches_base
                .clone()
                .into_iter()
                .filter_map(|mut hatch| {
                    hatch.translate_mut(center.x(), center.y());
                    hatch.rotate_around_point_mut(rot as f64, center);
                    let new_points = polygon
                        .lines_iter()
                        .filter_map(|line| line_intersection(hatch, line))
                        .filter_map(|intersection| match intersection {
                            LineIntersection::SinglePoint {
                                intersection,
                                is_proper: _,
                            } => Some(intersection),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    if new_points.len() != 2 {
                        None
                    } else {
                        Some(geo::Line {
                            start: new_points[0],
                            end: new_points[1],
                        })
                    }
                })
                //This filter is aesthetic: short lines in the hatching look bad.
                //However, it can lead to unwanted behavior: when a polygon is large, but cut to a
                //short and squat area by the map geometry, and the hatches align, it can leave
                //blank space in a large area.
                //TODO: fix
                .filter(|line| line.euclidean_length() > 7.)
                .collect();
            for line in hatches {
                self.hatching.add(Line::from_geo(line));
            }
        }
    }
}

#[derive(Clone)]
pub enum PolyOpType {
    Union,
    Subtraction,
}

pub struct PolyOp {
    base: MultiPolygon,
    operator: MultiPolygon,
    operation: PolyOpType,
}

impl PolyOp {
    pub fn new(kind: PolyOpType, other: MultiPolygon) -> Self {
        PolyOp {
            operator: other,
            base: MultiPolygon(vec![]),
            operation: kind,
        }
    }
}

impl Edit for PolyOp {
    type Target = Layer;
    type Output = ();

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self.operation {
            PolyOpType::Union => {
                self.base = target.area.intersection(&self.operator);
                target.area = target.area.union(&self.operator);
            }
            PolyOpType::Subtraction => {
                self.base = target.area.intersection(&self.operator);
                target.area = target.area.difference(&self.operator);
            }
        }
        target.update_sketch();
        target.generate_hatching();
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self.operation {
            PolyOpType::Union => {
                target.area = target.area.difference(&self.operator).union(&self.base);
            }
            PolyOpType::Subtraction => {
                target.area = target.area.union(&self.base);
            }
        }
        target.update_sketch();
        target.generate_hatching();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_undo_redo() {
        let mut test_layer = Layer::new();
        let mut history: History<_> = History::new();
        history.edit(
            &mut test_layer,
            PolyOp::new(
                PolyOpType::Union,
                MultiPolygon::new(vec![polygon![
                    (x: 0., y: 0.),
                    (x: 0., y: 1.),
                    (x: 1., y: 1.),
                    (x: 1., y: 0.)
                ]]),
            ),
        );
        let one = test_layer.clone();
        history.edit(
            &mut test_layer,
            PolyOp::new(
                PolyOpType::Union,
                MultiPolygon::new(vec![polygon![
                    (x: 1., y: 0.),
                    (x: 1., y: 0.5),
                    (x: 2., y: 0.5),
                    (x: 2., y: 0.)
                ]]),
            ),
        );
        let two = test_layer.clone();
        history.undo(&mut test_layer);
        let three = test_layer.clone();
        history.redo(&mut test_layer);
        let four = test_layer.clone();
        history.edit(
            &mut test_layer,
            PolyOp::new(
                PolyOpType::Subtraction,
                MultiPolygon::new(vec![polygon![
                    (x: 0., y: 0.),
                    (x: 0., y: 1.),
                    (x: 1., y: 1.),
                    (x: 1., y: 0.)
                ]]),
            ),
        );
        let five = test_layer.clone();
        history.undo(&mut test_layer);
        let six = test_layer.clone();
        history.redo(&mut test_layer);
        let seven = test_layer.clone();

        //Points are different so comparing whole layers would fail even if behavior is correct
        //I compare areas to ensure that the geometry makes sense without requiring the internals to be the same
        assert_eq!(one.area.unsigned_area(), three.area.unsigned_area());
        assert_eq!(two.area.unsigned_area(), four.area.unsigned_area());
        assert_eq!(four.area.unsigned_area(), six.area.unsigned_area());
        assert_eq!(five.area.unsigned_area(), seven.area.unsigned_area());
    }
}
