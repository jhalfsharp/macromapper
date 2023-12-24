use egui_macroquad::{egui, macroquad::prelude::*};
use std::collections::HashMap;

pub trait Frac {
    fn to_frac_string(&self) -> String;
}

impl Frac for f32 {
    //so uhh
    //im well aware that this implementation was terrible
    //i wrote it while on like two hours of sleep while i should have been doing schoolwork
    //that being said it works
    //if youre not me, and youre reading this, that means this code is open source
    //if this code bothers you, i accept pull requests
    fn to_frac_string(&self) -> String {
        let temp = self.to_string();
        let frac_names: HashMap<&'static str, &'static str> = HashMap::from([
            ("0.5", "1/2"),
            ("0.33333334", "1/3"),
            ("0.25", "1/4"),
            ("0.2", "1/5"),
            ("0.16666667", "1/6"),
        ]);
        frac_names
            .get(temp.as_str())
            .unwrap_or(&temp.as_str())
            .to_string()
    }
}

pub fn poly_rect(p1: Vec2, p2: Vec2) -> geo::MultiPolygon {
    geo::MultiPolygon::new(vec![geo::Polygon::new(
        geo::LineString::from(vec![
            (p1.x as f64, p1.y as f64),
            (p1.x as f64, p2.y as f64),
            (p2.x as f64, p2.y as f64),
            (p2.x as f64, p1.y as f64),
        ]),
        vec![],
    )])
}

#[derive(PartialEq)]
pub struct Cam {
    pub focus: Vec2,
    pub screen_rect: Rect,
    pub grid_rect: Rect,
    pub scale: f32,
    pub dpi: f32,
}

impl Cam {
    pub fn new(dpi_scale: f32) -> Self {
        Cam {
            focus: vec2(0., 0.),
            screen_rect: Default::default(),
            grid_rect: Default::default(),
            scale: 50.,
            dpi: dpi_scale,
        }
    }

    pub fn update_focus(&mut self, new: Vec2) {
        self.focus = new;
        self.grid_rect.x = self.focus.x - 0.5 * self.screen_rect.w;
        self.grid_rect.y = self.focus.y - 0.5 * self.screen_rect.h;
        self.grid_rect.w = self.screen_rect.w;
        self.grid_rect.h = self.screen_rect.h;
    }

    pub fn to_camera(&self) -> Camera2D {
        Camera2D {
            target: self.focus,
            zoom: vec2(2.0 / self.screen_rect.w, 2.0 / self.screen_rect.h),
            viewport: Some((
                (self.screen_rect.x * self.dpi) as i32,
                (self.screen_rect.y * self.dpi) as i32,
                (self.screen_rect.w * self.dpi) as i32,
                (self.screen_rect.h * self.dpi) as i32,
            )),
            ..Default::default()
        }
    }
}

//toggle_ui() and toggle() functions by Emil "emilk" Ernerfeldt, released under MIT license
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *on, ""));
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }
    response
}
pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}
