use super::{
    CROSSHAIR_RGBA, SELECTION_FILL_RGBA, highlight_marker_plot_position, highlight_mask_color,
    highlight_mask_plot_position,
};
use crate::{
    LineType, Size,
    camera::Camera,
    grid::TickWeight,
    plot_state::PlotState,
    plot_widget::{world_to_screen_position_x, world_to_screen_position_y},
    point::{MARKER_SIZE_WORLD, MarkerType},
    transform::{data_point_to_plot_with_transform, data_value_to_plot_with_axis_range},
};
use iced::{
    Color, Rectangle,
    widget::canvas::{self, Frame, Geometry},
};

fn rgba_to_color(rgba: [f32; 4]) -> Color {
    Color::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn selection_fill_color() -> Color {
    rgba_to_color(SELECTION_FILL_RGBA)
}

fn crosshair_color() -> Color {
    rgba_to_color(CROSSHAIR_RGBA)
}

fn marker_type_from_u32(marker: u32) -> MarkerType {
    match marker {
        0 => MarkerType::FilledCircle,
        1 => MarkerType::EmptyCircle,
        2 => MarkerType::Square,
        3 => MarkerType::Star,
        4 => MarkerType::Triangle,
        _ => MarkerType::FilledCircle,
    }
}

#[derive(Default)]
pub(crate) struct CanvasCaches {
    pub(crate) static_layer: canvas::Cache,
    pub(crate) overlay_layer: canvas::Cache,
}

pub(crate) fn draw(
    renderer: &iced::Renderer,
    caches: &CanvasCaches,
    state: &PlotState,
    bounds: Rectangle,
) -> Vec<Geometry> {
    let frame_bounds = Rectangle::with_size(bounds.size());

    let static_layer = caches
        .static_layer
        .draw_with_bounds(renderer, frame_bounds, |frame| {
            draw_grid(frame, state, bounds);
            draw_fills(frame, state, bounds);
            draw_lines(frame, state, bounds);
            draw_reference_lines(frame, state, bounds);
            draw_markers(frame, state, bounds);
        });

    let overlay_layer = caches
        .overlay_layer
        .draw_with_bounds(renderer, frame_bounds, |frame| {
            draw_highlights(frame, state, bounds);
            draw_selection(frame, state);
            draw_crosshairs(frame, state);
        });

    vec![static_layer, overlay_layer]
}

fn draw_grid(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for tick in &state.x_ticks {
        let color = match tick.tick.line_type {
            TickWeight::Major => state.grid_style.major,
            TickWeight::Minor => state.grid_style.minor,
            TickWeight::SubMinor => state.grid_style.sub_minor,
        };
        let line = canvas::Path::line(
            iced::Point::new(tick.screen_pos, 0.0),
            iced::Point::new(tick.screen_pos, bounds.height),
        );
        frame.stroke(&line, canvas::Stroke::default().with_color(color));
    }

    for tick in &state.y_ticks {
        let color = match tick.tick.line_type {
            TickWeight::Major => state.grid_style.major,
            TickWeight::Minor => state.grid_style.minor,
            TickWeight::SubMinor => state.grid_style.sub_minor,
        };
        let line = canvas::Path::line(
            iced::Point::new(0.0, tick.screen_pos),
            iced::Point::new(bounds.width, tick.screen_pos),
        );
        frame.stroke(&line, canvas::Stroke::default().with_color(color));
    }
}

fn draw_fills(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for fill in state.fills.iter() {
        for triangle in fill.vertices.chunks_exact(3) {
            let points = [
                world_to_canvas_point(triangle[0], &state.camera, &bounds),
                world_to_canvas_point(triangle[1], &state.camera, &bounds),
                world_to_canvas_point(triangle[2], &state.camera, &bounds),
            ];
            let path = canvas::Path::new(|builder| {
                builder.move_to(points[0]);
                builder.line_to(points[1]);
                builder.line_to(points[2]);
                builder.close();
            });
            frame.fill(&path, fill.color);
        }
    }
}

fn draw_lines(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for series in state.series.iter() {
        let Some(line_style) = series.line_style else {
            continue;
        };
        if series.len < 2 {
            continue;
        }

        let width = line_style.width.to_px(&state.camera, &bounds).max(0.5);
        let points = &state.points[series.start..series.start + series.len];
        let mut distance_along_strip = 0.0;

        for index in 1..points.len() {
            let break_segment = series
                .point_indices
                .get(index)
                .zip(series.point_indices.get(index - 1))
                .is_some_and(|(curr, prev)| *curr != *prev + 1);

            if break_segment {
                distance_along_strip = 0.0;
                continue;
            }

            let p0 = world_to_canvas_point(points[index - 1].position, &state.camera, &bounds);
            let p1 = world_to_canvas_point(points[index].position, &state.camera, &bounds);
            let delta = iced::Vector::new(p1.x - p0.x, p1.y - p0.y);
            let segment_length = (delta.x * delta.x + delta.y * delta.y).sqrt();
            if segment_length <= f32::EPSILON {
                continue;
            }

            let c0 = *state
                .point_colors
                .get(series.start + index - 1)
                .unwrap_or(&series.color);
            let c1 = *state
                .point_colors
                .get(series.start + index)
                .unwrap_or(&series.color);
            draw_styled_line_segment(
                frame,
                p0,
                p1,
                line_style.line_type,
                width,
                blend_colors(c0, c1),
                distance_along_strip,
            );
            distance_along_strip += segment_length;
        }
    }
}

fn draw_reference_lines(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for vline in state.vlines.iter() {
        let Some(vx_plot) = data_value_to_plot_with_axis_range(
            vline.x,
            state.x_axis_scale,
            vline.transform.as_ref(),
            Some(state.camera.x_range()),
        ) else {
            continue;
        };
        let Some(x) = world_to_screen_position_x(vx_plot, &state.camera, &bounds) else {
            continue;
        };
        draw_styled_line_segment(
            frame,
            iced::Point::new(x, 0.0),
            iced::Point::new(x, bounds.height),
            vline.line_style.line_type,
            vline
                .line_style
                .width
                .to_px(&state.camera, &bounds)
                .max(0.5),
            vline.color,
            0.0,
        );
    }

    for hline in state.hlines.iter() {
        let Some(hy_plot) = data_value_to_plot_with_axis_range(
            hline.y,
            state.y_axis_scale,
            hline.transform.as_ref(),
            Some(state.camera.y_range()),
        ) else {
            continue;
        };
        let Some(y) = world_to_screen_position_y(hy_plot, &state.camera, &bounds) else {
            continue;
        };
        draw_styled_line_segment(
            frame,
            iced::Point::new(0.0, y),
            iced::Point::new(bounds.width, y),
            hline.line_style.line_type,
            hline
                .line_style
                .width
                .to_px(&state.camera, &bounds)
                .max(0.5),
            hline.color,
            0.0,
        );
    }
}

fn draw_markers(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for series in state.series.iter() {
        if series.marker == u32::MAX {
            continue;
        }

        let end = series.start + series.len;
        for (local_index, point) in state.points[series.start..end].iter().enumerate() {
            let color = *state
                .point_colors
                .get(series.start + local_index)
                .unwrap_or(&series.color);
            draw_marker(
                frame,
                point.position,
                point.size,
                point.size_mode,
                series.marker,
                color,
                &state.camera,
                &bounds,
            );
        }
    }
}

fn draw_highlights(frame: &mut Frame, state: &PlotState, bounds: Rectangle) {
    for highlight in state.highlighted_points.iter() {
        if let Some(marker_style) = highlight.marker_style
            && let Some(plot_pos) = highlight_marker_plot_position(highlight, state)
        {
            let (size, size_mode) = marker_style.size.to_raw();
            draw_marker(
                frame,
                plot_pos,
                size,
                size_mode,
                marker_style.marker_type as u32,
                highlight.color,
                &state.camera,
                &bounds,
            );
        }

        let Some(mask_padding) = highlight.mask_padding else {
            continue;
        };
        let Some(marker_style) = highlight.marker_style else {
            continue;
        };
        let Some(plot_pos) = highlight_mask_plot_position(highlight, state) else {
            continue;
        };

        if marker_style.marker_type == MarkerType::Square
            && let Size::World(size) = marker_style.size
        {
            let Some(top_left_plot) = data_point_to_plot_with_transform(
                [highlight.x, highlight.y + size],
                state.x_axis_scale,
                state.y_axis_scale,
                &highlight.transform,
                Some(state.camera.axis_ranges()),
            ) else {
                continue;
            };
            let Some(bottom_right_plot) = data_point_to_plot_with_transform(
                [highlight.x + size, highlight.y],
                state.x_axis_scale,
                state.y_axis_scale,
                &highlight.transform,
                Some(state.camera.axis_ranges()),
            ) else {
                continue;
            };
            let top_left = world_to_canvas_point(top_left_plot, &state.camera, &bounds);
            let bottom_right = world_to_canvas_point(bottom_right_plot, &state.camera, &bounds);
            frame.fill_rectangle(
                iced::Point::new(
                    top_left.x.min(bottom_right.x),
                    top_left.y.min(bottom_right.y),
                ),
                iced::Size::new(
                    (bottom_right.x - top_left.x).abs(),
                    (bottom_right.y - top_left.y).abs(),
                ),
                highlight_mask_color(highlight.color),
            );
            continue;
        }

        let center = world_to_canvas_point(plot_pos, &state.camera, &bounds);
        let size = marker_style.size.to_px(&state.camera, &bounds) + mask_padding;
        let rect = canvas::Path::rectangle(
            iced::Point::new(center.x - size, center.y - size),
            iced::Size::new(size * 2.0, size * 2.0),
        );
        frame.fill(&rect, highlight_mask_color(highlight.color));
    }
}

fn draw_selection(frame: &mut Frame, state: &PlotState) {
    if !(state.selection.active || state.selection.moved) {
        return;
    }

    let start = state.selection.start;
    let end = state.selection.end;
    let top_left = iced::Point::new(start.x.min(end.x), start.y.min(end.y));
    let size = iced::Size::new((start.x - end.x).abs(), (start.y - end.y).abs());
    let rect = canvas::Path::rectangle(top_left, size);
    frame.fill(&rect, selection_fill_color());
}

fn draw_crosshairs(frame: &mut Frame, state: &PlotState) {
    if !state.crosshairs_enabled {
        return;
    }

    let pos = state.crosshairs_position;
    let horizontal = canvas::Path::line(
        iced::Point::new(0.0, pos.y),
        iced::Point::new(state.bounds.width, pos.y),
    );
    let vertical = canvas::Path::line(
        iced::Point::new(pos.x, 0.0),
        iced::Point::new(pos.x, state.bounds.height),
    );
    let stroke = canvas::Stroke::default().with_color(crosshair_color());
    frame.stroke(&horizontal, stroke);
    frame.stroke(&vertical, stroke);
}

#[allow(clippy::too_many_arguments)]
fn draw_marker(
    frame: &mut Frame,
    world_position: [f64; 2],
    size: f32,
    size_mode: u32,
    marker: u32,
    color: Color,
    camera: &Camera,
    bounds: &Rectangle,
) {
    let mut center_world = world_position;
    if size_mode == MARKER_SIZE_WORLD {
        let half = f64::from(size) * 0.5;
        center_world[0] += half;
        center_world[1] += half;
    }

    let center = world_to_canvas_point(center_world, camera, bounds);
    let radius = Size::size_px(size, size_mode, camera, bounds) * 0.5;
    let marker_type = marker_type_from_u32(marker);

    if marker_type == MarkerType::Square {
        if size_mode == MARKER_SIZE_WORLD {
            let top_left = world_to_canvas_point(
                [world_position[0], world_position[1] + f64::from(size)],
                camera,
                bounds,
            );
            let bottom_right = world_to_canvas_point(
                [world_position[0] + f64::from(size), world_position[1]],
                camera,
                bounds,
            );
            frame.fill_rectangle(
                iced::Point::new(
                    top_left.x.min(bottom_right.x),
                    top_left.y.min(bottom_right.y),
                ),
                iced::Size::new(
                    (bottom_right.x - top_left.x).abs(),
                    (bottom_right.y - top_left.y).abs(),
                ),
                color,
            );
        } else {
            let center = world_to_canvas_point(world_position, camera, bounds);
            let side = Size::size_px(size, size_mode, camera, bounds);
            let half = side * 0.5;
            frame.fill_rectangle(
                iced::Point::new(center.x - half, center.y - half),
                iced::Size::new(side, side),
                color,
            );
        }
        return;
    }

    let path = marker_path(center, radius, marker_type);
    match marker_type {
        MarkerType::EmptyCircle => {
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_width((radius * 0.3).max(1.0))
                    .with_color(color),
            );
        }
        _ => frame.fill(&path, color),
    }
}

fn stroke_segment(frame: &mut Frame, p0: iced::Point, p1: iced::Point, width: f32, color: Color) {
    let path = canvas::Path::line(p0, p1);
    frame.stroke(
        &path,
        canvas::Stroke::default()
            .with_width(width)
            .with_color(color),
    );
}

fn draw_styled_line_segment(
    frame: &mut Frame,
    p0: iced::Point,
    p1: iced::Point,
    line_type: LineType,
    width: f32,
    color: Color,
    phase_start: f32,
) {
    let delta = iced::Vector::new(p1.x - p0.x, p1.y - p0.y);
    let length = (delta.x * delta.x + delta.y * delta.y).sqrt();
    if length <= f32::EPSILON {
        return;
    }

    let direction = iced::Vector::new(delta.x / length, delta.y / length);

    match line_type {
        LineType::Solid => stroke_segment(frame, p0, p1, width, color),
        LineType::Dashed { length: dash } => {
            let dash = dash.max(1.0);
            let gap = (dash * 0.5).max(1.0);
            let pattern = dash + gap;
            let mut pattern_start = -(phase_start.rem_euclid(pattern));
            while pattern_start < length {
                let seg_start = pattern_start.max(0.0);
                let seg_end = (pattern_start + dash).min(length);
                if seg_end > seg_start {
                    let start = iced::Point::new(
                        p0.x + direction.x * seg_start,
                        p0.y + direction.y * seg_start,
                    );
                    let end = iced::Point::new(
                        p0.x + direction.x * seg_end,
                        p0.y + direction.y * seg_end,
                    );
                    stroke_segment(frame, start, end, width, color);
                }
                pattern_start += pattern;
            }
        }
        LineType::Dotted { spacing } => {
            let spacing = spacing.max(1.0);
            let step = spacing * 2.0;
            let radius = width.max(1.5) * 0.5;
            let remainder = phase_start.rem_euclid(step);
            let mut offset = if remainder <= f32::EPSILON {
                0.0
            } else {
                step - remainder
            };
            while offset <= length {
                let center =
                    iced::Point::new(p0.x + direction.x * offset, p0.y + direction.y * offset);
                let dot = canvas::Path::circle(center, radius);
                frame.fill(&dot, color);
                offset += step;
            }
        }
    }
}

fn blend_colors(a: Color, b: Color) -> Color {
    Color::from_rgba(
        (a.r + b.r) * 0.5,
        (a.g + b.g) * 0.5,
        (a.b + b.b) * 0.5,
        (a.a + b.a) * 0.5,
    )
}

fn marker_path(center: iced::Point, radius: f32, marker_type: MarkerType) -> canvas::Path {
    match marker_type {
        MarkerType::FilledCircle | MarkerType::EmptyCircle => canvas::Path::circle(center, radius),
        MarkerType::Square => canvas::Path::rectangle(
            iced::Point::new(center.x - radius, center.y - radius),
            iced::Size::new(radius * 2.0, radius * 2.0),
        ),
        MarkerType::Triangle => canvas::Path::new(|builder| {
            builder.move_to(iced::Point::new(center.x, center.y - radius));
            builder.line_to(iced::Point::new(center.x + radius, center.y + radius));
            builder.line_to(iced::Point::new(center.x - radius, center.y + radius));
            builder.close();
        }),
        MarkerType::Star => canvas::Path::new(|builder| {
            for index in 0..5 {
                let outer_angle =
                    -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / 5.0;
                let inner_angle = outer_angle + std::f32::consts::TAU / 10.0;
                let outer = iced::Point::new(
                    center.x + outer_angle.cos() * radius,
                    center.y + outer_angle.sin() * radius,
                );
                let inner = iced::Point::new(
                    center.x + inner_angle.cos() * radius * 0.45,
                    center.y + inner_angle.sin() * radius * 0.45,
                );
                if index == 0 {
                    builder.move_to(outer);
                } else {
                    builder.line_to(outer);
                }
                builder.line_to(inner);
            }
            builder.close();
        }),
    }
}

fn world_to_canvas_point(world: [f64; 2], camera: &Camera, bounds: &Rectangle) -> iced::Point {
    let ndc_x = (world[0] - camera.position.x) / camera.half_extents.x;
    let ndc_y = (world[1] - camera.position.y) / camera.half_extents.y;
    iced::Point::new(
        (ndc_x as f32 + 1.0) * 0.5 * bounds.width,
        (1.0 - ndc_y as f32) * 0.5 * bounds.height,
    )
}
