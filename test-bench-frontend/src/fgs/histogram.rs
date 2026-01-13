use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Render a histogram to a canvas element.
pub fn render_histogram(
    canvas_id: &str,
    info_id: &str,
    histogram: &[u32],
    mean: f64,
    max_bin: u16,
    log_scale: bool,
) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let canvas = match document.get_element_by_id(canvas_id) {
        Some(el) => match el.dyn_into::<HtmlCanvasElement>() {
            Ok(c) => c,
            Err(_) => return,
        },
        None => return,
    };

    let ctx = match canvas.get_context("2d") {
        Ok(Some(ctx)) => match ctx.dyn_into::<CanvasRenderingContext2d>() {
            Ok(c) => c,
            Err(_) => return,
        },
        _ => return,
    };

    let width = canvas.width() as f64;
    let height = canvas.height() as f64;

    ctx.set_fill_style_str("#000000");
    ctx.fill_rect(0.0, 0.0, width, height);

    if histogram.is_empty() {
        return;
    }

    let transform_value = |v: f64| -> f64 {
        if log_scale {
            if v > 0.0 {
                (v + 1.0).ln()
            } else {
                0.0
            }
        } else {
            v
        }
    };

    let max_value = histogram
        .iter()
        .map(|&v| transform_value(v as f64))
        .fold(0.0_f64, f64::max);

    if max_value == 0.0 {
        return;
    }

    let num_bins = histogram.len();
    let bar_width = width / num_bins as f64;

    ctx.set_fill_style_str("#00aa00");

    for (i, &count) in histogram.iter().enumerate() {
        let transformed = transform_value(count as f64);
        let bar_height = (transformed / max_value) * height;
        let x = i as f64 * bar_width;
        let y = height - bar_height;
        ctx.fill_rect(x, y, bar_width.max(1.0), bar_height);
    }

    let scale_label = if log_scale { " (log)" } else { "" };
    if let Some(info_el) = document.get_element_by_id(info_id) {
        info_el.set_inner_html(&format!(
            "Mean: {mean:.1} | Max bin: {max_bin}{scale_label}"
        ));
    }
}
