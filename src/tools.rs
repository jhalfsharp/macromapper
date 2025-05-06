use core::mem;

use crate::{drawing::*, utils::*};
use egui_macroquad::macroquad::prelude::*;
use new_egui_macroquad as egui_macroquad;

pub trait Tool {
    fn left_click(&mut self, pos: Vec2, layer: usize, op_type: &PolyOpType) -> Option<MapEdit>;
    fn right_click(&mut self, pos: Vec2) -> Option<MapEdit>;
    fn drag(&mut self, mouse_new: Vec2, mouse_old: Vec2, camera: &mut Cam) -> Option<MapEdit>;
    fn preview(&mut self, pos: Vec2, thickness: f32, color: Color) -> Sketch;
}

#[derive(PartialEq)]
pub struct DragTool {}

impl Tool for DragTool {
    fn left_click(&mut self, _pos: Vec2, _layer: usize, _op_type: &PolyOpType) -> Option<MapEdit> {
        None
    }
    fn right_click(&mut self, _pos: Vec2) -> Option<MapEdit> {
        None
    }
    fn drag(&mut self, mouse_new: Vec2, mouse_old: Vec2, camera: &mut Cam) -> Option<MapEdit> {
        camera.update_focus(
            camera.focus + (mouse_new - mouse_old) * (Vec2::NEG_X + Vec2::Y) / camera.scale,
        );
        None
    }
    fn preview(&mut self, _pos: Vec2, thickness: f32, color: Color) -> Sketch {
        Sketch::new(thickness, color)
    }
}

#[derive(PartialEq)]
pub struct RectTool {
    point: Option<Vec2>, // first part of rectangle (none when not in use)
}

impl RectTool {
    pub fn new() -> Self {
        RectTool { point: None }
    }
}

impl Tool for RectTool {
    fn left_click(&mut self, pos: Vec2, layer: usize, op_type: &PolyOpType) -> Option<MapEdit> {
        match self.point {
            Some(_) => {
                let out = poly_rect(self.point.expect("there should be a first point"), pos);
                self.point = None;
                match op_type {
                    PolyOpType::Union => Some(MapEdit::Union(MapUnion::new(layer, out))),
                    PolyOpType::Subtraction => {
                        Some(MapEdit::Subtraction(MapSubtraction::new(layer, out)))
                    }
                }
            }
            None => {
                self.point = Some(pos);
                None
            }
        }
    }
    fn right_click(&mut self, _pos: Vec2) -> Option<MapEdit> {
        self.point = None;
        None
    }
    fn drag(&mut self, _mouse_new: Vec2, _mouse_old: Vec2, _camera: &mut Cam) -> Option<MapEdit> {
        None
    }
    fn preview(&mut self, pos: Vec2, thickness: f32, color: Color) -> Sketch {
        let mut out = Sketch::new(thickness, color);
        if self.point.is_some() {
            let point = self.point.unwrap();
            out.add(Line::new(point.x, point.y, pos.x, point.y));
            out.add(Line::new(point.x, pos.y, pos.x, pos.y));
            out.add(Line::new(point.x, point.y, point.x, pos.y));
            out.add(Line::new(pos.x, point.y, pos.x, pos.y));
        }
        out
    }
}

pub struct PolyTool {
    points: Vec<Vec2>,
}

impl PolyTool {
    pub fn new() -> Self {
        PolyTool { points: vec![] }
    }
}

impl Tool for PolyTool {
    fn left_click(&mut self, pos: Vec2, layer: usize, op_type: &PolyOpType) -> Option<MapEdit> {
        if self.points.is_empty() {
            self.points.push(pos);
            return None;
        }
        if self.points.first().is_some_and(|p| *p == pos)
            || self.points.last().is_some_and(|p| *p == pos)
        {
            let points = mem::replace(&mut self.points, vec![]);
            let coords = points
                .into_iter()
                .map(|v| geo::Coord {
                    x: v.x as f64,
                    y: v.y as f64,
                })
                .collect::<Vec<_>>();
            let polygon = geo::MultiPolygon::new(vec![geo::Polygon::new(
                geo::LineString::from(coords),
                vec![],
            )]);
            return match op_type {
                PolyOpType::Union => Some(MapEdit::Union(MapUnion::new(layer, polygon))),
                PolyOpType::Subtraction => {
                    Some(MapEdit::Subtraction(MapSubtraction::new(layer, polygon)))
                }
            };
        }
        if self.points.contains(&pos) {
            let index = self.points.iter().position(|p| *p == pos).unwrap();
            self.points.truncate(index + 1);
        } else {
            self.points.push(pos);
        }
        None
    }

    fn right_click(&mut self, _pos: Vec2) -> Option<MapEdit> {
        self.points.clear();
        None
    }

    fn drag(&mut self, _mouse_new: Vec2, _mouse_old: Vec2, _camera: &mut Cam) -> Option<MapEdit> {
        None
    }

    fn preview(&mut self, pos: Vec2, thickness: f32, color: Color) -> Sketch {
        let mut out = Sketch::new(thickness, color);
        self.points.push(pos);
        for pair in self.points.windows(2) {
            out.add(Line::new(pair[0].x, pair[0].y, pair[1].x, pair[1].y));
        }
        self.points.pop();
        out
    }
}
