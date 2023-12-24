use crate::{drawing::*, utils::*};
use egui_macroquad::macroquad::prelude::*;

pub trait Tool {
    fn left_click(&mut self, pos: Vec2, op_type: &PolyOpType) -> Option<PolyOp>;
    fn right_click(&mut self, pos: Vec2) -> Option<PolyOp>;
    fn drag(&mut self, mouse_new: Vec2, mouse_old: Vec2, camera: &mut Cam) -> Option<PolyOp>;
    fn preview(&mut self, pos: Vec2, thickness: f32, color: Color) -> Sketch;
}

#[derive(PartialEq)]
pub struct DragTool {}

impl Tool for DragTool {
    fn left_click(&mut self, _pos: Vec2, _op_type: &PolyOpType) -> Option<PolyOp> {
        None
    }
    fn right_click(&mut self, _pos: Vec2) -> Option<PolyOp> {
        None
    }
    fn drag(&mut self, mouse_new: Vec2, mouse_old: Vec2, camera: &mut Cam) -> Option<PolyOp> {
        camera.update_focus(camera.focus + (mouse_new - mouse_old) * (Vec2::NEG_X + Vec2::Y));
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
    fn left_click(&mut self, pos: Vec2, op_type: &PolyOpType) -> Option<PolyOp> {
        match self.point {
            Some(_) => {
                let out = poly_rect(self.point.expect("there should be a first point"), pos);
                self.point = None;
                Some(PolyOp::new(op_type.clone(), out))
            }
            None => {
                self.point = Some(pos);
                None
            }
        }
    }
    fn right_click(&mut self, _pos: Vec2) -> Option<PolyOp> {
        self.point = None;
        None
    }
    fn drag(&mut self, _mouse_new: Vec2, _mouse_old: Vec2, _camera: &mut Cam) -> Option<PolyOp> {
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
