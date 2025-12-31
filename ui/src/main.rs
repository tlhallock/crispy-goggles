use common::grpc::shape_events_client::ShapeEventsClient;
use common::grpc::{CreateShapeRequest, Event, SubscribeRequest};

use common::model::{Coord, OrientedPoint, TIME_PER_SECOND};
use futures::StreamExt;
use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tonic::Request;
use tonic_web_wasm_client::Client;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};

use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum RenderError {
    MissingTasks,
    MissingStartTime,
    InvalidTime,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::MissingTasks => write!(f, "No tasks available for rendering"),
            RenderError::MissingStartTime => write!(f, "Missing start time for rendering"),
            RenderError::InvalidTime => write!(f, "Invalid time value encountered"),
        }
    }
}

impl Error for RenderError {}

#[derive(Clone, Copy, Debug)]
struct TimeSync {
    wall_time_ms: u64,
    game_time_ms: u64,
}

#[derive(Clone, Copy, Debug)]
struct ZoomState {
    center_x: f64, // in meters
    center_y: f64, // in meters
    pixels_per_meter: f64,
}

impl Default for ZoomState {
    fn default() -> Self {
        ZoomState {
            center_x: 0.0,
            center_y: 0.0,
            pixels_per_meter: 100.0, // Default zoom level
        }
    }
}

impl ZoomState {
    /// Maps a point from game coordinates (meters) to screen coordinates (pixels)
    fn map_to_pixel(&self, point_meters: (f64, f64), canvas: &HtmlCanvasElement) -> (f64, f64) {
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        // Translate from world coordinates to canvas coordinates
        let dx = (point_meters.0 - self.center_x) * self.pixels_per_meter;
        let dy = (point_meters.1 - self.center_y) * self.pixels_per_meter;

        // Canvas center
        let cx = canvas_width / 2.0;
        let cy = canvas_height / 2.0;

        (cx + dx, cy + dy)
    }

    /// Maps a point from screen coordinates (pixels) to game coordinates (meters)
    fn pixel_to_map(&self, pixel: (f64, f64), canvas: &HtmlCanvasElement) -> (f64, f64) {
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        let cx = canvas_width / 2.0;
        let cy = canvas_height / 2.0;

        let dx = (pixel.0 - cx) / self.pixels_per_meter;
        let dy = (pixel.1 - cy) / self.pixels_per_meter;

        (self.center_x + dx, self.center_y + dy)
    }
}

#[derive(Clone, Copy, Debug)]
struct DrawingRect {
    start_x: f64,   // in meters
    start_y: f64,   // in meters
    current_x: f64, // in meters
    current_y: f64, // in meters
}

#[derive(Default)]
struct UiState {
    sync: Option<TimeSync>,
    anims: HashMap<u64, common::grpc::Animatable>,
    zoom: ZoomState,
    drawing_rect: Option<DrawingRect>,
}

fn window() -> Window {
    web_sys::window().expect("no global `window`")
}

fn wall_now_ms() -> u64 {
    js_sys::Date::now() as u64
}

fn game_now_ms(sync: Option<TimeSync>) -> Option<u64> {
    let s = sync?;
    let now = wall_now_ms();
    Some(
        s.game_time_ms
            .saturating_add(now.saturating_sub(s.wall_time_ms)),
    )
}

fn get_ctx(canvas_id: &str) -> CanvasRenderingContext2d {
    // todo: remove unwraps
    let doc = window().document().unwrap();
    let canvas = doc.get_element_by_id(canvas_id).unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into().unwrap();
    canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap()
}

fn get_canvas(canvas_id: &str) -> HtmlCanvasElement {
    let doc = window().document().unwrap();
    let canvas = doc.get_element_by_id(canvas_id).unwrap();
    canvas.dyn_into().unwrap()
}

fn set_color(ctx: &CanvasRenderingContext2d, rgb: (u8, u8, u8)) {
    let css = format!("rgb({}, {}, {})", rgb.0, rgb.1, rgb.2);
    // ctx.set_fill_style(&css.clone().into());
    // ctx.set_stroke_style(&css.into());
    ctx.set_fill_style(&wasm_bindgen::JsValue::from_str(&css));
    ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str(&css));
}

fn clear(ctx: &CanvasRenderingContext2d, canvas: &HtmlCanvasElement) {
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    ctx.clear_rect(0.0, 0.0, width, height);
}

fn draw_grid(ctx: &CanvasRenderingContext2d, canvas: &HtmlCanvasElement, zoom: &ZoomState) {
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;

    // Fill background with green
    ctx.set_fill_style(&wasm_bindgen::JsValue::from_str("#4a7c4a"));
    ctx.fill_rect(0.0, 0.0, width, height);

    // Calculate the visible range in meters
    let top_left = zoom.pixel_to_map((0.0, 0.0), canvas);
    let bottom_right = zoom.pixel_to_map((width, height), canvas);

    // Determine grid spacing based on zoom level
    let meter_spacing = 1.0; // Draw grid every 1 meter

    // Find the first grid line to draw (round down to nearest meter)
    let start_x = (top_left.0 / meter_spacing).floor() * meter_spacing;
    let start_y = (top_left.1 / meter_spacing).floor() * meter_spacing;

    // Draw vertical lines
    let mut x = start_x;
    while x <= bottom_right.0 {
        let (screen_x, _) = zoom.map_to_pixel((x, 0.0), canvas);

        // Every 10 meters, draw a thicker line
        let is_major = (x / 10.0).round() * 10.0 == x;

        if is_major {
            ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(255, 255, 255, 0.3)"));
            ctx.set_line_width(2.0);
        } else {
            ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str(
                "rgba(255, 255, 255, 0.15)",
            ));
            ctx.set_line_width(1.0);
        }

        ctx.begin_path();
        ctx.move_to(screen_x, 0.0);
        ctx.line_to(screen_x, height);
        ctx.stroke();

        x += meter_spacing;
    }

    // Draw horizontal lines
    let mut y = start_y;
    while y <= bottom_right.1 {
        let (_, screen_y) = zoom.map_to_pixel((0.0, y), canvas);

        // Every 10 meters, draw a thicker line
        let is_major = (y / 10.0).round() * 10.0 == y;

        if is_major {
            ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(255, 255, 255, 0.3)"));
            ctx.set_line_width(2.0);
        } else {
            ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str(
                "rgba(255, 255, 255, 0.15)",
            ));
            ctx.set_line_width(1.0);
        }

        ctx.begin_path();
        ctx.move_to(0.0, screen_y);
        ctx.line_to(width, screen_y);
        ctx.stroke();

        y += meter_spacing;
    }

    // Reset line width
    ctx.set_line_width(1.0);
}

fn place_for(
    path: &[common::grpc::PathSegment],
    t_game: u64,
) -> Result<OrientedPoint, RenderError> {
    // if the binary search doesn't work out...
    // for segment in path.iter().rev() {
    //     if segment.begin_time <= t_game {
    //         return eval_segment(segment, t_game);
    //     }
    // }

    let first = path.first().ok_or(RenderError::MissingTasks)?;
    // if t_game < first.begin_time {
    //     return Err(RenderError::InvalidTime);
    // }
    // Could check if it is the first, always...

    match path.binary_search_by_key(&t_game, |seg| seg.begin_time) {
        Ok(idx) => eval_segment(&path[idx], t_game),
        Err(0) => first
            .begin_location
            .map(|p| OrientedPoint {
                point: common::model::Point {
                    x: p.x as Coord,
                    y: p.y as Coord,
                },
                orientation: first.begin_orientation,
            })
            .ok_or(RenderError::MissingStartTime),
        Err(idx) => eval_segment(&path[idx - 1], t_game),
    }
}

fn eval_segment(
    seg: &common::grpc::PathSegment,
    t_game: u64,
) -> Result<OrientedPoint, RenderError> {
    let begin_location = seg
        .begin_location
        .ok_or_else(|| RenderError::MissingStartTime)?;
    let d_t = t_game.saturating_sub(seg.begin_time) as f64;

    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
        "location = {:?}, delta = {:?}, d_t = {}",
        begin_location, seg.delta, d_t
    )));

    // Assuming delta.dx and delta.dy are in meters per second
    Ok(OrientedPoint {
        point: common::model::Point {
            x: begin_location.x
                + (seg
                    .delta
                    .map_or(0.0, |d| d.dx as f64 * (d_t / TIME_PER_SECOND as f64)))
                    as Coord,
            y: begin_location.y
                + (seg
                    .delta
                    .map_or(0.0, |d| d.dy as f64 * (d_t / TIME_PER_SECOND as f64)))
                    as Coord,
        },
        orientation: seg.begin_orientation + seg.d_orientation.unwrap_or(0.0),
    })
}

fn draw_anim(
    ctx: &CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
    anim: &common::grpc::Animatable,
    t_game: u64,
    zoom: &ZoomState,
) -> Result<(), RenderError> {
    // color
    let rgb = anim
        .color
        .as_ref()
        .map(|c| (c.r as u8, c.g as u8, c.b as u8))
        .unwrap_or((0, 0, 0));
    set_color(ctx, rgb);

    let fill = anim.fill;

    let oriented_point = place_for(&anim.path, t_game)?;
    // Convert from meters to pixels using zoom state
    let (x, y) = zoom.map_to_pixel(
        (oriented_point.point.x as f64, oriented_point.point.y as f64),
        canvas,
    );
    let shape = anim.shape.as_ref().ok_or(RenderError::MissingTasks)?;
    let sk = shape.kind.as_ref().ok_or(RenderError::MissingTasks)?;

    match sk {
        common::grpc::shape::Kind::Circle(c) => {
            ctx.begin_path();
            let radius_pixels = c.radius as f64 * zoom.pixels_per_meter;
            let _ = ctx.arc(x, y, radius_pixels, 0.0, std::f64::consts::TAU);
            if fill {
                ctx.fill();
            } else {
                ctx.stroke();
            }
        }
        common::grpc::shape::Kind::Rectangle(r) => {
            let w_pixels = r.w as f64 * zoom.pixels_per_meter;
            let h_pixels = r.h as f64 * zoom.pixels_per_meter;
            if fill {
                ctx.fill_rect(x, y, w_pixels, h_pixels);
            } else {
                ctx.stroke_rect(x, y, w_pixels, h_pixels);
            }
        }
    }
    Ok(())
}

fn draw_rectangle_overlay(
    ctx: &CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
    rect: &DrawingRect,
    zoom: &ZoomState,
) {
    // Convert both points from meters to pixels
    let (x1, y1) = zoom.map_to_pixel((rect.start_x, rect.start_y), canvas);
    let (x2, y2) = zoom.map_to_pixel((rect.current_x, rect.current_y), canvas);

    let x = x1.min(x2);
    let y = y1.min(y2);
    let w = (x1 - x2).abs();
    let h = (y1 - y2).abs();

    // Draw semi-transparent rectangle
    ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(255, 0, 0, 0.8)"));
    ctx.set_fill_style(&wasm_bindgen::JsValue::from_str("rgba(255, 0, 0, 0.2)"));
    ctx.set_line_width(2.0);
    ctx.stroke_rect(x, y, w, h);
    ctx.fill_rect(x, y, w, h);
    ctx.set_line_width(1.0);
}

fn apply_event(state: &mut UiState, ev: &Event) {
    let Some(kind) = &ev.kind else {
        return;
    };

    match kind {
        common::grpc::event::Kind::Synchronize(s) => {
            state.sync = Some(TimeSync {
                wall_time_ms: s.wall_time,
                game_time_ms: s.game_time,
            });
        }
        common::grpc::event::Kind::Show(show) => {
            if let Some(anim) = &show.anim {
                state.anims.insert(anim.id, anim.clone());
            }
        }
        common::grpc::event::Kind::Update(upd) => {
            if let Some(a) = state.anims.get_mut(&upd.id) {
                a.path = upd.path.clone();
            }
        }
        common::grpc::event::Kind::Hide(h) => {
            state.anims.remove(&h.id);
        }
        _ => {}
    }
}

fn start_animation_loop(
    ctx: CanvasRenderingContext2d,
    canvas: HtmlCanvasElement,
    shared: Rc<RefCell<UiState>>,
) {
    // Standard self-rescheduling RAF closure pattern:
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_ts: f64| {
        // Resize canvas to match display size to prevent distortion
        let display_width = canvas.offset_width() as u32;
        let display_height = canvas.offset_height() as u32;
        if canvas.width() != display_width || canvas.height() != display_height {
            canvas.set_width(display_width);
            canvas.set_height(display_height);
        }

        // Compute current game time

        let t_game = {
            let st = shared.borrow();

            // web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(

            // we should never hit wall_now_ms...
            game_now_ms(st.sync).unwrap_or_else(wall_now_ms)
        };

        // Redraw
        clear(&ctx, &canvas);
        {
            let st = shared.borrow();
            // Draw grid first
            draw_grid(&ctx, &canvas, &st.zoom);

            // Then draw animations on top
            for anim in st.anims.values() {
                if let Err(e) = draw_anim(&ctx, &canvas, anim, t_game, &st.zoom) {
                    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                        "Render error for anim id {}: {}",
                        anim.id, e
                    )));
                }
            }

            // Draw rectangle overlay if currently drawing
            if let Some(rect) = &st.drawing_rect {
                draw_rectangle_overlay(&ctx, &canvas, rect, &st.zoom);
            }
        }

        // schedule next frame
        // TODO: there is an unwrap...
        let _ =
            window().request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }) as Box<dyn FnMut(f64)>));

    // Kick it off
    let _ = window().request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());

    // TODO: do not leak it...
    // Important: we intentionally leak the closure by keeping it in Rc<RefCell<Option<...>>>
    // tied to this function’s captured environment. In practice this is fine for app lifetime.
}

async fn grpc_client() -> ShapeEventsClient<Client> {
    let client = Client::new("http://127.0.0.1:50051".into());
    ShapeEventsClient::new(client)
}

#[component]
fn App() -> impl IntoView {
    let (status, set_status) = signal::<String>("Starting…".to_string());
    let (bounds_display, set_bounds_display) = signal::<String>("".to_string());
    let (bounds_update_trigger, set_bounds_update_trigger) = signal(0u32);

    // Non-reactive shared state (fast updates; avoids rerendering on every event/frame)
    let shared = Rc::new(RefCell::new(UiState::default()));

    // Track mouse dragging state
    let (is_dragging, set_is_dragging) = signal(false);
    let (last_mouse_pos, set_last_mouse_pos) = signal::<Option<(f64, f64)>>(None);
    let (is_drawing_rect, set_is_drawing_rect) = signal(false);

    // Update bounds display when trigger changes
    {
        let shared_for_bounds = shared.clone();
        Effect::new(move |_| {
            // React to the trigger
            let _ = bounds_update_trigger.get();

            let canvas = get_canvas("canvas");
            let state = shared_for_bounds.borrow();
            let width = canvas.width() as f64;
            let height = canvas.height() as f64;

            let top_left = state.zoom.pixel_to_map((0.0, 0.0), &canvas);
            let bottom_right = state.zoom.pixel_to_map((width, height), &canvas);

            let bounds_text = format!(
                "View: X: {:.2}m to {:.2}m, Y: {:.2}m to {:.2}m ({}x{} m)",
                top_left.0,
                bottom_right.0,
                top_left.1,
                bottom_right.1,
                (bottom_right.0 - top_left.0).abs() as i32,
                (bottom_right.1 - top_left.1).abs() as i32
            );
            set_bounds_display.set(bounds_text);
        });
    }

    {
        let shared_for_effect = shared.clone();
        Effect::new(move |_| {
            let ctx = get_ctx("canvas");
            let canvas = get_canvas("canvas");
            clear(&ctx, &canvas);

            // Start continuous redraw loop (only once per mount is fine; in CSR this Effect runs once)
            start_animation_loop(ctx.clone(), canvas, shared_for_effect.clone());

            // IMPORTANT: do NOT move shared_for_effect into the async block directly.
            // Clone it *inside* so the Effect closure remains FnMut.
            spawn_local({
                let shared_for_task = shared_for_effect.clone();
                async move {
                    set_status.set("Connecting gRPC-Web…".into());

                    let mut client = grpc_client().await;

                    let resp = match client.subscribe(Request::new(SubscribeRequest {})).await {
                        Ok(r) => r,
                        Err(e) => {
                            set_status.set(format!("Subscribe failed: {e}"));
                            return;
                        }
                    };

                    set_status.set("Subscribed.".into());

                    let mut stream = resp.into_inner();
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(ev) => {
                                let mut st = shared_for_task.borrow_mut();
                                apply_event(&mut st, &ev);
                            }
                            Err(e) => {
                                set_status.set(format!("Stream error: {e}"));
                                break;
                            }
                        }
                    }
                }
            });
        });
    }

    let on_make_shape = move |_| {
        spawn_local(async move {
            let mut client = grpc_client().await;
            match client
                .create_shape(Request::new(CreateShapeRequest {}))
                .await
            {
                Ok(resp) => {
                    let id = resp.into_inner().id;
                    set_status.set(format!("CreateShape ok (id={id})"));
                }
                Err(e) => {
                    set_status.set(format!("CreateShape failed: {e}"));
                }
            }
        });
    };

    // Mouse event handlers for pan and zoom
    let shared_for_mouse = shared.clone();
    let shared_for_mouse_down = shared_for_mouse.clone();
    let on_mouse_down = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            // Left button - pan
            set_is_dragging.set(true);
            set_last_mouse_pos.set(Some((ev.offset_x() as f64, ev.offset_y() as f64)));
        } else if ev.button() == 2 {
            // Right button - start drawing rectangle
            ev.prevent_default();
            set_is_drawing_rect.set(true);

            let canvas = get_canvas("canvas");
            let scale_x = canvas.width() as f64 / canvas.offset_width() as f64;
            let scale_y = canvas.height() as f64 / canvas.offset_height() as f64;

            let mouse_x = ev.offset_x() as f64 * scale_x;
            let mouse_y = ev.offset_y() as f64 * scale_y;

            let mut state = shared_for_mouse_down.borrow_mut();
            let (start_x, start_y) = state.zoom.pixel_to_map((mouse_x, mouse_y), &canvas);

            state.drawing_rect = Some(DrawingRect {
                start_x,
                start_y,
                current_x: start_x,
                current_y: start_y,
            });
        }
    };

    let shared_for_mouse_move = shared_for_mouse.clone();
    let on_mouse_move = move |ev: web_sys::MouseEvent| {
        let canvas = get_canvas("canvas");
        let scale_x = canvas.width() as f64 / canvas.offset_width() as f64;
        let scale_y = canvas.height() as f64 / canvas.offset_height() as f64;

        if is_dragging.get() {
            // Handle panning
            if let Some((last_x, last_y)) = last_mouse_pos.get() {
                let current_x = ev.offset_x() as f64;
                let current_y = ev.offset_y() as f64;

                let dx_pixels = (current_x - last_x) * scale_x;
                let dy_pixels = (current_y - last_y) * scale_y;

                let mut state = shared_for_mouse_move.borrow_mut();

                // Convert pixel delta to meters
                let dx_meters = dx_pixels / state.zoom.pixels_per_meter;
                let dy_meters = dy_pixels / state.zoom.pixels_per_meter;

                // Move the center (opposite direction of drag)
                state.zoom.center_x -= dx_meters;
                state.zoom.center_y -= dy_meters;

                set_last_mouse_pos.set(Some((current_x, current_y)));
                drop(state); // Release borrow before triggering update
                set_bounds_update_trigger.update(|v| *v += 1);
            }
        } else if is_drawing_rect.get() {
            // Handle rectangle drawing
            let mouse_x = ev.offset_x() as f64 * scale_x;
            let mouse_y = ev.offset_y() as f64 * scale_y;

            let mut state = shared_for_mouse_move.borrow_mut();
            let (current_x, current_y) = state.zoom.pixel_to_map((mouse_x, mouse_y), &canvas);

            if let Some(rect) = &mut state.drawing_rect {
                rect.current_x = current_x;
                rect.current_y = current_y;
            }
        }
    };

    let shared_for_mouse_up = shared_for_mouse.clone();
    let on_mouse_up = move |ev: web_sys::MouseEvent| {
        if ev.button() == 0 {
            set_is_dragging.set(false);
            set_last_mouse_pos.set(None);
        } else if ev.button() == 2 && is_drawing_rect.get() {
            // Finish drawing rectangle
            set_is_drawing_rect.set(false);

            let mut state = shared_for_mouse_up.borrow_mut();
            if let Some(rect) = state.drawing_rect.take() {
                let min_x = rect.start_x.min(rect.current_x);
                let max_x = rect.start_x.max(rect.current_x);
                let min_y = rect.start_y.min(rect.current_y);
                let max_y = rect.start_y.max(rect.current_y);
                let width = max_x - min_x;
                let height = max_y - min_y;
                let center_x = (min_x + max_x) / 2.0;
                let center_y = (min_y + max_y) / 2.0;

                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "Rectangle drawn - Center: ({:.2}m, {:.2}m), Width: {:.2}m, Height: {:.2}m, Bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}]",
                    center_x, center_y, width, height, min_x, max_x, min_y, max_y
                )));
            }
        }
    };

    let shared_for_wheel = shared_for_mouse.clone();
    let on_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();

        let canvas = get_canvas("canvas");
        let mut state = shared_for_wheel.borrow_mut();

        // Get mouse position relative to canvas (in CSS pixels)
        let mouse_x_css = ev.offset_x() as f64;
        let mouse_y_css = ev.offset_y() as f64;

        // Scale from CSS pixels to canvas pixels
        let scale_x = canvas.width() as f64 / canvas.offset_width() as f64;
        let scale_y = canvas.height() as f64 / canvas.offset_height() as f64;

        let mouse_x = mouse_x_css * scale_x;
        let mouse_y = mouse_y_css * scale_y;

        // Convert mouse position to game coordinates before zoom
        let point_before = state.zoom.pixel_to_map((mouse_x, mouse_y), &canvas);

        // Adjust zoom
        let zoom_factor = if ev.delta_y() < 0.0 { 1.1 } else { 0.9 };
        state.zoom.pixels_per_meter *= zoom_factor;

        // Clamp zoom to reasonable values
        state.zoom.pixels_per_meter = state.zoom.pixels_per_meter.clamp(1.0, 1000.0);

        // Convert mouse position to game coordinates after zoom
        let point_after = state.zoom.pixel_to_map((mouse_x, mouse_y), &canvas);

        // Adjust center to keep the point under the mouse the same
        state.zoom.center_x += point_before.0 - point_after.0;
        state.zoom.center_y += point_before.1 - point_after.1;

        drop(state); // Release borrow before triggering update
        set_bounds_update_trigger.update(|v| *v += 1);
    };

    view! {
        <div style="font-family: sans-serif; padding: 12px;">
            <h2>"Shapes (gRPC-Web)"</h2>
            <button on:click=on_make_shape style="padding: 8px 12px;">
                "Make a shape"
            </button>
            <span style="margin-left: 12px;">{move || status.get()}</span>
            <div style="margin-top: 8px; font-family: monospace; font-size: 12px; color: #666;">
                {move || bounds_display.get()}
            </div>
            <div style="margin-top: 8px;">
                <canvas
                    id="canvas"
                    width="900"
                    height="600"
                    style=move || format!(
                        "border: 1px solid #ccc; width: 100%; height: 80vh; cursor: {};",
                        if is_dragging.get() { "grabbing" } else { "grab" }
                    )
                    on:mousedown=on_mouse_down
                    on:mousemove=on_mouse_move
                    on:mouseup=on_mouse_up.clone()
                    on:mouseleave=on_mouse_up
                    on:wheel=on_wheel
                    on:contextmenu=move |ev: web_sys::MouseEvent| ev.prevent_default()
                ></canvas>
            </div>
        </div>
    }
}

pub fn main() {
    mount_to_body(|| view! { <App/> })
}
