use common::grpc::shape_events_client::ShapeEventsClient;
use common::grpc::{CreateShapeRequest, Event, SubscribeRequest};

use futures::StreamExt;
use leptos::prelude::*;
use leptos::task::spawn_local;
use tonic::Request;
use tonic_web_wasm_client::Client;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[derive(Clone, Copy, Debug)]
struct TimeSync {
    wall_time_ms: u64,
    game_time_ms: u64,
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
    let window = web_sys::window().unwrap();
    let doc = window.document().unwrap();
    let canvas = doc.get_element_by_id(canvas_id).unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into().unwrap();
    canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap()
}

fn set_color(ctx: &CanvasRenderingContext2d, rgb: (u8, u8, u8)) {
    let css = format!("rgb({}, {}, {})", rgb.0, rgb.1, rgb.2);
    ctx.set_fill_style(&css.clone().into());
    ctx.set_stroke_style(&css.into());
}

fn clear(ctx: &CanvasRenderingContext2d) {
    // match your canvas size
    ctx.clear_rect(0.0, 0.0, 900.0, 600.0);
}

fn default_place_for_id(id: u64) -> (f64, f64) {
    let x = ((id as f64 * 97.0) % 850.0) + 25.0;
    let y = ((id as f64 * 53.0) % 550.0) + 25.0;
    (x, y)
}

fn place_for(id: u64, path: &[common::grpc::PathSegment], t_game: u64) -> (f64, f64) {
    if path.is_empty() {
        return default_place_for_id(id);
    }

    if t_game <= path[0].begin_time {
        return (path[0].begin_x as f64, path[0].begin_y as f64);
    }

    let mut last = &path[0];
    for seg in path {
        last = seg;

        let b = seg.begin_time;
        let has_end_time = seg.has_end_time;
        let e = if has_end_time { seg.end_time } else { b };

        if t_game < b {
            break;
        }

        if t_game <= e {
            return eval_segment(seg, t_game);
        }
    }

    if last.has_end_location {
        (last.end_x as f64, last.end_y as f64)
    } else {
        (last.begin_x as f64, last.begin_y as f64)
    }
}

fn eval_segment(seg: &common::grpc::PathSegment, t_game: u64) -> (f64, f64) {
    let bx = seg.begin_x as f64;
    let by = seg.begin_y as f64;

    // If we don’t have both end_location and end_time, treat as a point.
    if !(seg.has_end_location && seg.has_end_time) {
        return (bx, by);
    }

    let b = seg.begin_time;
    let e = seg.end_time;

    // Avoid divide-by-zero / malformed segments.
    if e <= b {
        return if seg.has_end_location {
            (seg.end_x as f64, seg.end_y as f64)
        } else {
            (bx, by)
        };
    }

    let ex = seg.end_x as f64;
    let ey = seg.end_y as f64;

    let alpha = (t_game.saturating_sub(b) as f64) / ((e - b) as f64);
    let alpha = alpha.clamp(0.0, 1.0);

    (bx + alpha * (ex - bx), by + alpha * (ey - by))
}

/// Draw one Show(anim). This expects your proto matches the earlier shapes.proto:
/// - Event.kind = Begin|Show|Update|Hide
/// - Show.anim contains Animatable { id, shape, fill, color, path }
fn draw_show(ctx: &CanvasRenderingContext2d, ev: &Event) {
    let Some(kind) = &ev.kind else {
        return;
    };

    match kind {
        common::grpc::event::Kind::Synchronize(s) => {
            // store in a signal / Rc<RefCell<Option<TimeSync>>> etc
            current_sync.set(Some(TimeSync {
                wall_time_ms: s.wall_time,
                game_time_ms: s.game_time,
            }));
        }
        common::grpc::event::Kind::Show(show) => {
            let Some(anim) = &show.anim else {
                return;
            };
            let id = anim.id;

            // color
            let rgb = anim
                .color
                .as_ref()
                .map(|c| (c.r as u8, c.g as u8, c.b as u8))
                // todo: missing color case
                .unwrap_or((0, 0, 0));
            set_color(ctx, rgb);

            let fill = anim.fill;
            let (x, y) = place_for_id(id);

            // shape
            let Some(shape) = &anim.shape else {
                return;
            };
            let Some(sk) = &shape.kind else {
                return;
            };

            match sk {
                common::grpc::shape::Kind::Circle(c) => {
                    ctx.begin_path();
                    let _ = ctx.arc(x, y, c.radius as f64, 0.0, std::f64::consts::TAU);
                    if fill {
                        ctx.fill();
                    } else {
                        ctx.stroke();
                    }
                }
                common::grpc::shape::Kind::Rectangle(r) => {
                    if fill {
                        ctx.fill_rect(x, y, r.w as f64, r.h as f64);
                    } else {
                        ctx.stroke_rect(x, y, r.w as f64, r.h as f64);
                    }
                }
            }
        }
        _ => {
            // For now we ignore Begin/Update/Hide
            // When you implement Hide, you’ll want to keep a local map of animatables and redraw.
        }
    }
}

async fn grpc_client() -> ShapeEventsClient<Client> {
    // This is the gRPC-Web endpoint (same as server listening addr).
    // IMPORTANT: must be http:// not https:// for localhost dev unless you set up TLS.
    let client = Client::new("http://127.0.0.1:50051".into());
    ShapeEventsClient::new(client)
}

#[component]
fn App() -> impl IntoView {
    // We’ll hold a simple “status” string + a counter.
    let (status, set_status) = signal::<String>("Starting…".to_string());
    let (count, set_count) = signal::<u64>(0);

    // Start subscription exactly once after mount.
    // We draw events directly to the canvas context.
    Effect::new(move |_| {
        // Run once
        let ctx = get_ctx("canvas");
        clear(&ctx);

        spawn_local(async move {
            set_status.set("Connecting gRPC-Web…".into());

            let mut client = grpc_client().await;

            let req = Request::new(SubscribeRequest {});
            let resp = match client.subscribe(req).await {
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
                        draw_show(&ctx, &ev);
                    }
                    Err(e) => {
                        set_status.set(format!("Stream error: {e}"));
                        break;
                    }
                }
            }
        });
    });

    let on_make_shape = move |_| {
        spawn_local(async move {
            let mut client = grpc_client().await;

            match client
                .create_shape(Request::new(CreateShapeRequest {}))
                .await
            {
                Ok(resp) => {
                    let id = resp.into_inner().id;
                    set_count.update(|c| *c += 1);
                    set_status.set(format!(
                        "CreateShape ok (id={id}). total clicks={}",
                        count.get_untracked()
                    ));
                }
                Err(e) => {
                    set_status.set(format!("CreateShape failed: {e}"));
                }
            }
        });
    };

    view! {
        <div style="font-family: sans-serif; padding: 12px;">
            <h2>"Shapes (gRPC-Web)"</h2>
            <button on:click=on_make_shape style="padding: 8px 12px;">
                "Make a shape"
            </button>
            <span style="margin-left: 12px;">{move || status.get()}</span>
            <div style="margin-top: 12px;">
                <canvas id="canvas" width="900" height="600" style="border: 1px solid #ccc;"></canvas>
            </div>
        </div>
    }
}

pub fn main() {
    leptos::mount::mount_to_body(|| view! { <App/> })
}
