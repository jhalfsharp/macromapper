use egui_macroquad::egui::util::hash;
use egui_macroquad::macroquad::{
    prelude::*,
    rand::{rand, srand},
};
use fast_poisson::Poisson2D;
use geo::line_intersection::line_intersection;
use geo::*;
use new_egui_macroquad as egui_macroquad;
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
    area_sketch: Sketch,
    hatching: Sketch,
    name: String,
}

impl Layer {
    pub fn new(name: String) -> Self {
        Layer {
            area: MultiPolygon(vec![]),
            area_sketch: Sketch::new(3.0, BLACK),
            hatching: Sketch::new(2.0, GRAY),
            name,
        }
    }
    pub fn draw(&self) {
        self.hatching.draw();
        self.area_sketch.draw();
    }
    fn update_sketch(&mut self) {
        self.area_sketch.clear();
        for l in self.area.lines_iter() {
            self.area_sketch.add(Line::from_geo(l));
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
            let rot = rand() % 180;
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
pub struct Map {
    layers: Vec<Layer>,
}

impl Map {
    pub fn new() -> Self {
        Map { layers: vec![] }
    }
    pub fn append_layer(&mut self) {
        self.layers.push(Layer::new(
            "layer-".to_string() + &self.layers.len().to_string(),
        ));
    }
    pub fn layers_iter(&self) -> core::slice::Iter<'_, Layer> {
        self.layers.iter()
    }
}

pub enum MapEdit {
    Union(MapUnion),
    Subtraction(MapSubtraction),
}

//boring boilerplate to make things work
//check edit and undo methods for each operation for implementation details
impl Edit for MapEdit {
    type Target = Map;
    type Output = ();

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MapEdit::Union(u) => u.edit(target),
            MapEdit::Subtraction(s) => s.edit(target),
        }
    }
    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MapEdit::Union(u) => u.undo(target),
            MapEdit::Subtraction(s) => s.undo(target),
        }
    }
}

pub struct MapUnion {
    base: MultiPolygon,
    operator: MultiPolygon,
    layer: usize,
}

impl MapUnion {
    pub fn new(layer: usize, operator: MultiPolygon) -> Self {
        Self {
            base: MultiPolygon(vec![]),
            operator,
            layer,
        }
    }
    fn edit(&mut self, target: &mut Map) {
        let target_layer = target
            .layers
            .get_mut(self.layer)
            .expect("layer should exist");
        self.base = target_layer.area.intersection(&self.operator);
        target_layer.area = target_layer.area.union(&self.operator);
        target_layer.update_sketch();
        target_layer.generate_hatching();
    }
    fn undo(&mut self, target: &mut Map) {
        let target_layer = target
            .layers
            .get_mut(self.layer)
            .expect("layer should exist");
        target_layer.area = target_layer
            .area
            .difference(&self.operator)
            .union(&self.base);
        target_layer.update_sketch();
        target_layer.generate_hatching();
    }
}

pub struct MapSubtraction {
    base: MultiPolygon,
    operator: MultiPolygon,
    layer: usize,
}

impl MapSubtraction {
    pub fn new(layer: usize, operator: MultiPolygon) -> Self {
        Self {
            base: MultiPolygon(vec![]),
            operator,
            layer,
        }
    }
    fn edit(&mut self, target: &mut Map) {
        let target_layer = target
            .layers
            .get_mut(self.layer)
            .expect("layer should exist");
        self.base = target_layer.area.intersection(&self.operator);
        target_layer.area = target_layer.area.difference(&self.operator);
        target_layer.update_sketch();
        target_layer.generate_hatching();
    }
    fn undo(&mut self, target: &mut Map) {
        let target_layer = target
            .layers
            .get_mut(self.layer)
            .expect("layer should exist");
        target_layer.area = target_layer.area.union(&self.base);
        target_layer.update_sketch();
        target_layer.generate_hatching();
    }
}

#[derive(Clone)]
pub enum PolyOpType {
    Union,
    Subtraction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_undo_redo() {
        let mut test_map: Map = Map {
            layers: vec![Layer::new("test".to_string())],
        };
        let mut history: History<_> = History::new();
        history.edit(
            &mut test_map,
            MapEdit::Union(MapUnion::new(
                0,
                MultiPolygon::new(vec![polygon![
                    (x: 0., y: 0.),
                    (x: 0., y: 1.),
                    (x: 1., y: 1.),
                    (x: 1., y: 0.)
                ]]),
            )),
        );
        let one = test_map.clone();
        history.edit(
            &mut test_map,
            MapEdit::Union(MapUnion::new(
                0,
                MultiPolygon::new(vec![polygon![
                    (x: 1., y: 0.),
                    (x: 1., y: 0.5),
                    (x: 2., y: 0.5),
                    (x: 2., y: 0.)
                ]]),
            )),
        );
        let two = test_map.clone();
        history.undo(&mut test_map);
        let three = test_map.clone();
        history.redo(&mut test_map);
        let four = test_map.clone();
        history.edit(
            &mut test_map,
            MapEdit::Union(MapUnion::new(
                0,
                MultiPolygon::new(vec![polygon![
                    (x: 0., y: 0.),
                    (x: 0., y: 1.),
                    (x: 1., y: 1.),
                    (x: 1., y: 0.)
                ]]),
            )),
        );
        let five = test_map.clone();
        history.undo(&mut test_map);
        let six = test_map.clone();
        history.redo(&mut test_map);
        let seven = test_map.clone();

        //Points are different so comparing whole layers would fail even if behavior is correct
        //I compare areas to ensure that the geometry makes sense without requiring the internals to be the same
        assert_eq!(
            one.layers.get(0).unwrap().area.unsigned_area(),
            three.layers.get(0).unwrap().area.unsigned_area()
        );
        assert_eq!(
            two.layers.get(0).unwrap().area.unsigned_area(),
            four.layers.get(0).unwrap().area.unsigned_area()
        );
        assert_eq!(
            four.layers.get(0).unwrap().area.unsigned_area(),
            six.layers.get(0).unwrap().area.unsigned_area()
        );
        assert_eq!(
            five.layers.get(0).unwrap().area.unsigned_area(),
            seven.layers.get(0).unwrap().area.unsigned_area()
        );
    }
}
