use geo::*;
use macroquad::prelude::*;
use undo::*;

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
    pub fn add(&mut self, line: Line) -> () {
        self.lines.push(line);
    }
    pub fn draw(&self) -> () {
        self.lines
            .iter()
            .for_each(|i| i.draw(self.thickness, self.color))
    }
    pub fn clear(&mut self) -> () {
        self.lines.clear();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line(Vec2, Vec2);

impl Line {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Line(vec2(x1, y1), vec2(x2, y2))
    }
    pub fn from_geo(line: geo::Line) -> Self {
        Line(
            vec2(line.start_point().x() as f32, line.start_point().y() as f32),
            vec2(line.end_point().x() as f32, line.end_point().y() as f32),
        )
    }
    pub fn draw(&self, thickness: f32, color: Color) -> () {
        draw_line(self.0.x, self.0.y, self.1.x, self.1.y, thickness, color);
        draw_circle(self.0.x, self.0.y, thickness / 2., color);
        draw_circle(self.1.x, self.1.y, thickness / 2., color);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    area: MultiPolygon,
    sketch: Sketch,
}

impl Layer {
    pub fn new() -> Self {
        Layer {
            area: MultiPolygon(vec![]),
            sketch: Sketch::new(3.0, BLACK),
        }
    }
    pub fn draw(&self) -> () {
        self.sketch.draw();
    }
    fn update_sketch(&mut self) -> () {
        self.sketch.clear();
        for l in self.area.lines_iter() {
            self.sketch.add(Line::from_geo(l));
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
