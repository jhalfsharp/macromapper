use camera::mouse;
use egui_extras::{install_image_loaders, Size, StripBuilder};
use egui_macroquad::egui::{self, emath::RectTransform, include_image, Image, Layout, Visuals};
use egui_macroquad::macroquad::{self, input, prelude::*};
use ico::*;
use new_egui_macroquad as egui_macroquad;
use undo::Record;

mod drawing;
use drawing::*;
mod utils;
use utils::*;
mod tools;
use tools::*;

const GRID_SIZE: f32 = 50.;

//Window setup
fn default_conf() -> Conf {
    let file = std::io::Cursor::new(include_bytes!("../assets/logo/macromapper.ico"));
    //i use unwrap here like 7 times and i guess technically its bad practice
    //but the icon isnt gonna change so it shouldnt panic if it launches once
    //anyway yeah sue me
    let dir = IconDir::read(file).unwrap();
    Conf {
        window_title: "Macromapper".to_string(),
        window_width: 1200,
        window_height: 675,
        high_dpi: true,
        icon: Some(miniquad::conf::Icon {
            big: dir.entries()[0]
                .decode()
                .unwrap()
                .rgba_data()
                .try_into()
                .unwrap(),
            medium: dir.entries()[1]
                .decode()
                .unwrap()
                .rgba_data()
                .try_into()
                .unwrap(),
            small: dir.entries()[2]
                .decode()
                .unwrap()
                .rgba_data()
                .try_into()
                .unwrap(),
        }),
        sample_count: 8,
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(default_conf)]
async fn main() {
    //Mouse globals
    let mut mouse_old = vec2(0., 0.);
    let mut mouse_new: Vec2;
    let mut is_dragging: bool = false;
    let mut mouse_pressed_old: bool = false;
    let mut mouse_pressed_new: bool;
    let mut mouse_pressed_r: bool;
    let mut mouse_in_egui: bool = false;
    let mut mouse_grid: Vec2;
    let mut mouse_grid_snapped: Vec2;
    let mut drag_started: Vec2 = vec2(0., 0.);
    let mut snap: f32 = 0.5;

    //Camera globals
    // let res_scale: f32; //screen DPI scale (usually 1.0)
    // unsafe {
    //     //I really wish I didn't have to use any unsafe, but apparently the DPI scale is dangerous
    //     res_scale = get_internal_gl().quad_context.dpi_scale();
    // }
    let mut camera = Cam::new(1.0);

    //Image loading and rasterizing
    //again, images shouldnt change so unwrap is safe if it launches once
    let drag_img = "file://assets/icons/d_drag.svg";
    let rect_img = "file://assets/icons/d_rect.svg";
    let poly_img = "file://assets/icons/d_pen.svg";

    let zoom_in_img = "file://assets/icons/z_zoom_in.svg";
    let zoom_out_img = "file://assets/icons/z_zoom_out.svg";

    let zoom_sizes = vec![
        0.1, 0.2, 0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.1, 1.2, 1.25, 1.33, 1.5, 1.75, 2.0,
    ];

    //App state globals
    let mut tool: Box<dyn Tool> = Box::new(DragTool {});
    let mut selected_tool: i8 = 1;
    let mut tool_type = true;

    let mut active_map = Map::new();
    active_map.append_layer();
    let mut history = Record::<MapEdit>::new();
    let mut active_layer: usize = 0;

    loop {
        egui_macroquad::ui(|egui_ctx| {
            install_image_loaders(egui_ctx);

            egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
                // The top panel is often a good place for a menu bar:
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            ui.close_menu();
                        }
                        ui.menu_button("Preferences", |ui| {
                            if ui.button("Dark mode").clicked() {
                                egui_ctx.set_visuals(Visuals::dark());
                            }
                            if ui.button("Light mode").clicked() {
                                egui_ctx.set_visuals(Visuals::light());
                            }
                        });
                    });
                    ui.menu_button("View", |ui| {
                        if ui.button("Go to origin").clicked() {
                            camera.update_focus(vec2(0., 0.));
                            ui.close_menu();
                        }
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.add(egui::ImageButton::new(zoom_in_img)).clicked() {
                            camera.scale = *zoom_sizes
                                .iter()
                                .find(|&&e| e > camera.scale)
                                .unwrap_or_else(|| &camera.scale);
                        }
                        if ui.add(egui::ImageButton::new(zoom_out_img)).clicked() {
                            camera.scale = *zoom_sizes
                                .iter()
                                .rfind(|&&e| e < camera.scale)
                                .unwrap_or_else(|| &camera.scale);
                        }
                        ui.add(egui::Slider::new(&mut camera.scale, 0.1..=2.0).text("Zoom"));
                    });
                });
            });
            egui::TopBottomPanel::bottom("bottom_panel").show(egui_ctx, |ui| {
                ui.label("Test");
            });
            egui::SidePanel::left("left_panel").show(egui_ctx, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(35.0))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            if ui
                                .add(egui::ImageButton::new(drag_img).selected(selected_tool == 1))
                                .clicked()
                            {
                                tool = Box::new(DragTool {});
                                selected_tool = 1;
                            }
                            if ui
                                .add(egui::ImageButton::new(rect_img).selected(selected_tool == 2))
                                .clicked()
                            {
                                tool = Box::new(RectTool::new());
                                selected_tool = 2;
                            }
                            if ui
                                .add(egui::ImageButton::new(poly_img).selected(selected_tool == 3))
                                .clicked()
                            {
                                tool = Box::new(PolyTool::new());
                                selected_tool = 3;
                            }
                        });

                        strip.cell(|ui| {
                            ui.label("Test");
                            ui.horizontal(|ui| {
                                ui.add(toggle(&mut tool_type)).labelled_by(
                                    ui.label(if tool_type { "Draw" } else { "Erase" }).id,
                                );
                            });
                            egui::ComboBox::from_label("Snap fraction")
                                .selected_text(snap.to_frac_string())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut snap, 1.0, "1");
                                    ui.selectable_value(&mut snap, 0.5, "1/2");
                                    ui.selectable_value(&mut snap, 1.0 / 3.0, "1/3");
                                    ui.selectable_value(&mut snap, 0.25, "1/4");
                                    ui.selectable_value(&mut snap, 0.2, "1/5");
                                    ui.selectable_value(&mut snap, 1.0 / 6.0, "1/6");
                                });
                            //Fill with empty space to allow resizing
                            //ui.allocate_space(ui.available_size());
                        })
                    });
            });
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("Test");
                ui.group(|ui| {
                    ui.label("Layer example");
                });
            });
            let available = egui_ctx.available_rect();
            let screen_to_relative = RectTransform::from_to(
                egui_ctx.used_rect(),
                egui::Rect::from_x_y_ranges(0.0..=screen_width(), 0.0..=screen_height()),
            );
            let canvas = screen_to_relative.transform_rect(available);
            //Update main rect
            if camera.screen_rect.w != canvas.width() || camera.screen_rect.h != canvas.height() {
                camera.screen_rect.x = canvas.left();
                camera.screen_rect.y = screen_height() - canvas.bottom();
                camera.screen_rect.w = canvas.width();
                camera.screen_rect.h = canvas.height();
            }
            mouse_in_egui = egui_ctx.is_using_pointer();
        });

        camera.update_focus(camera.focus);

        // Process keys, mouse etc.
        mouse_new = mouse_position().into();
        mouse_pressed_new = is_mouse_button_down(MouseButton::Left);
        mouse_pressed_r = is_mouse_button_down(MouseButton::Right);

        //This could be shorter but that sacrifices clarity
        if mouse_pressed_new && !mouse_in_egui {
            if !mouse_pressed_old && camera.screen_rect.contains(mouse_new) {
                drag_started = mouse_new;
                is_dragging = true;
            }
        } else {
            is_dragging = false;
        }

        // Handle coordinates
        mouse_grid = (mouse_new - camera.screen_rect.point()) * vec2(1.0, -1.0) / camera.scale
            + camera.grid_rect.point()
            + vec2(0.0, camera.grid_rect.h);
        mouse_grid_snapped = (mouse_grid / GRID_SIZE / snap).round() * GRID_SIZE * snap;

        //Update based on input
        if is_dragging {
            tool.drag(mouse_new, mouse_old, &mut camera);
        }

        if !mouse_pressed_new
            && mouse_pressed_old
            && camera.screen_rect.contains(mouse_new)
            && !mouse_in_egui
            && drag_started == mouse_old
        {
            if let Some(i) = tool.left_click(
                mouse_grid_snapped,
                active_layer,
                if tool_type {
                    &PolyOpType::Union
                } else {
                    &PolyOpType::Subtraction
                },
            ) {
                history.edit(&mut active_map, i);
            }
        }

        if mouse_pressed_r && camera.screen_rect.contains(mouse_new) && !mouse_in_egui {
            if let Some(i) = tool.right_click(mouse_grid_snapped) {
                history.edit(&mut active_map, i);
            }
        }

        if input::is_key_down(KeyCode::LeftControl) || input::is_key_down(KeyCode::RightControl) {
            if input::is_key_pressed(KeyCode::Z) {
                //undo: ctrl-z
                history.undo(&mut active_map);
            }
            if input::is_key_pressed(KeyCode::Y) {
                //redo: ctrl-y
                history.redo(&mut active_map);
            }
        }

        mouse_old = mouse_new;
        mouse_pressed_old = mouse_pressed_new;

        //Set up camera
        set_camera(&camera.to_camera());

        //Grid
        clear_background(WHITE);
        let mut grid = Sketch::new(1., LIGHTGRAY);
        ((camera.grid_rect.left() / GRID_SIZE) as i32
            ..=(camera.grid_rect.right() / GRID_SIZE) as i32)
            .map(|i| {
                grid.add(Line::new(
                    i as f32 * GRID_SIZE,
                    camera.grid_rect.top(),
                    i as f32 * GRID_SIZE,
                    camera.grid_rect.bottom(),
                ));
            })
            .count();
        ((camera.grid_rect.top() / GRID_SIZE) as i32
            ..=(camera.grid_rect.bottom() / GRID_SIZE) as i32)
            .map(|i| {
                grid.add(Line::new(
                    camera.grid_rect.left(),
                    i as f32 * GRID_SIZE,
                    camera.grid_rect.right(),
                    i as f32 * GRID_SIZE,
                ));
            })
            .count();
        grid.draw();
        for l in active_map.layers_iter() {
            l.draw();
        }

        //draw_circle(0., 0., 20., YELLOW);

        //Draw snapped cursor circle
        draw_circle(mouse_grid_snapped.x, mouse_grid_snapped.y, 3.0, RED);

        tool.preview(mouse_grid_snapped, 1., RED).draw();

        egui_macroquad::draw();
        // Draw things after egui

        next_frame().await;
    }
}
