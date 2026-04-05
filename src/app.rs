use crate::osm::load_graph_from_pbf;
use crate::router::AStarRouter;
use crate::types::*;
use crate::ui::{draw_panel, Button};
use macroquad::prelude::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct NavigationApp {
    pub graph: RoadGraph,
    pub route: Option<RouteResult>,
    pub start_node: Option<NodeId>,
    pub goal_node: Option<NodeId>,
    pub options: RoutingOptions,
    pub click_mode: ClickMode,
    pub hovered_node: Option<NodeId>,
    pub status_text: String,
    pub is_loading: bool,
    pub load_rx: Option<Receiver<Result<RoadGraph, String>>>,
    pub draw_nodes: bool,

    pub bounds: Option<GeoBounds>,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub dragging: bool,
    pub last_mouse_x: f32,
    pub last_mouse_y: f32,
}

impl NavigationApp {
    pub fn new(pbf_path: &str) -> Self {
        let (tx, rx) = mpsc::channel::<Result<RoadGraph, String>>();
        let path = pbf_path.to_string();

        thread::spawn(move || {
            let result = load_graph_from_pbf(&path);
            let _ = tx.send(result);
        });

        Self {
            graph: RoadGraph::new(),
            route: None,
            start_node: None,
            goal_node: None,
            options: RoutingOptions::default(),
            click_mode: ClickMode::SelectStart,
            hovered_node: None,
            status_text: format!("Loading {}", pbf_path),
            is_loading: true,
            load_rx: Some(rx),
            draw_nodes: false,

            bounds: None,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            dragging: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
        }
    }

    pub fn update(&mut self) {
        self.poll_loader();

        if !self.is_loading && !self.graph.nodes.is_empty() {
            self.handle_camera_input();
            self.hovered_node = self.find_nearest_node_to_mouse();
        } else {
            self.hovered_node = None;
        }
    }

    fn poll_loader(&mut self) {
        if !self.is_loading {
            return;
        }

        let Some(rx) = &self.load_rx else {
            return;
        };

        match rx.try_recv() {
            Ok(result) => {
                self.is_loading = false;
                self.load_rx = None;

                match result {
                    Ok(graph) => {
                        let node_count = graph.nodes.len();
                        let edge_count = graph.edges.len();

                        self.graph = graph;
                        self.bounds = self.compute_bounds();
                        self.reset_camera();

                        self.start_node = if node_count > 0 { Some(0) } else { None };
                        self.goal_node = if node_count > 1 { Some(node_count - 1) } else { None };
                        self.status_text =
                            format!("Loaded: {} nodes, {} edges", node_count, edge_count);

                        if self.start_node.is_some() && self.goal_node.is_some() {
                            self.recalculate_route();
                        }
                    }
                    Err(err) => {
                        self.status_text = err;
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.is_loading = false;
                self.load_rx = None;
                self.status_text = "Loader thread failed".to_string();
            }
        }
    }

    fn compute_bounds(&self) -> Option<GeoBounds> {
        if self.graph.nodes.is_empty() {
            return None;
        }

        let min_lat = self
            .graph
            .nodes
            .iter()
            .map(|n| n.point.lat)
            .fold(f64::INFINITY, f64::min);

        let max_lat = self
            .graph
            .nodes
            .iter()
            .map(|n| n.point.lat)
            .fold(f64::NEG_INFINITY, f64::max);

        let min_lon = self
            .graph
            .nodes
            .iter()
            .map(|n| n.point.lon)
            .fold(f64::INFINITY, f64::min);

        let max_lon = self
            .graph
            .nodes
            .iter()
            .map(|n| n.point.lon)
            .fold(f64::NEG_INFINITY, f64::max);

        Some(GeoBounds {
            min_lat,
            max_lat,
            min_lon,
            max_lon,
        })
    }

    fn reset_camera(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    fn handle_camera_input(&mut self) {
        let (mx, my) = mouse_position();

        if self.is_inside_map(mx, my) {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y.abs() > 0.0 {
                let old_zoom = self.zoom;
                self.zoom = (self.zoom * (1.0 + wheel_y * 0.12)).clamp(0.2, 20.0);

                if (self.zoom - old_zoom).abs() > f32::EPSILON {
                    let rect = self.map_rect();
                    let cx = rect.x + rect.w * 0.5;
                    let cy = rect.y + rect.h * 0.5;

                    self.pan_x = (self.pan_x - (mx - cx) * 0.1 * wheel_y).clamp(-5000.0, 5000.0);
                    self.pan_y = (self.pan_y - (my - cy) * 0.1 * wheel_y).clamp(-5000.0, 5000.0);
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) && self.is_inside_map(mx, my) {
            self.dragging = true;
            self.last_mouse_x = mx;
            self.last_mouse_y = my;
        }

        if self.dragging && is_mouse_button_down(MouseButton::Right) {
            let dx = mx - self.last_mouse_x;
            let dy = my - self.last_mouse_y;

            self.pan_x += dx;
            self.pan_y += dy;

            self.last_mouse_x = mx;
            self.last_mouse_y = my;
        }

        if is_mouse_button_released(MouseButton::Right) {
            self.dragging = false;
        }

        if is_key_pressed(KeyCode::R) {
            self.reset_camera();
        }
    }

    pub fn handle_input(&mut self) {
        if self.is_loading {
            return;
        }

        let buttons = self.buttons();

        if buttons.mode_fastest.clicked() {
            self.options.mode = RouteMode::Fastest;
            self.recalculate_route();
        }
        if buttons.mode_shortest.clicked() {
            self.options.mode = RouteMode::Shortest;
            self.recalculate_route();
        }
        if buttons.select_start.clicked() {
            self.click_mode = ClickMode::SelectStart;
        }
        if buttons.select_goal.clicked() {
            self.click_mode = ClickMode::SelectGoal;
        }
        if buttons.toggle_tolls.clicked() {
            self.options.avoid_tolls = !self.options.avoid_tolls;
            self.recalculate_route();
        }
        if buttons.toggle_motorway.clicked() {
            self.options.avoid_motorways = !self.options.avoid_motorways;
            self.recalculate_route();
        }
        if buttons.toggle_traffic.clicked() {
            self.options.traffic_enabled = !self.options.traffic_enabled;
            self.recalculate_route();
        }
        if buttons.route.clicked() {
            self.recalculate_route();
        }

        if is_key_pressed(KeyCode::N) {
            self.draw_nodes = !self.draw_nodes;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();

            if self.is_inside_map(mx, my) {
                if let Some(node_id) = self.find_nearest_node_to_mouse() {
                    match self.click_mode {
                        ClickMode::SelectStart => self.start_node = Some(node_id),
                        ClickMode::SelectGoal => self.goal_node = Some(node_id),
                    }
                    self.recalculate_route();
                }
            }
        }

        if is_key_pressed(KeyCode::Key1) {
            self.click_mode = ClickMode::SelectStart;
        }
        if is_key_pressed(KeyCode::Key2) {
            self.click_mode = ClickMode::SelectGoal;
        }
        if is_key_pressed(KeyCode::Enter) {
            self.recalculate_route();
        }
    }

    pub fn draw(&self) {
        self.draw_top_bar();
        self.draw_side_panel();
        self.draw_map();
    }

    fn recalculate_route(&mut self) {
        let (Some(start), Some(goal)) = (self.start_node, self.goal_node) else {
            self.route = None;
            return;
        };

        if self.graph.nodes.is_empty() {
            self.route = None;
            return;
        }

        self.route = Some(AStarRouter::find_route(
            &self.graph,
            start,
            goal,
            &self.options,
        ));
    }

    fn draw_top_bar(&self) {
        draw_panel(0.0, 0.0, SCREEN_WIDTH, TOP_BAR_HEIGHT);

        draw_text(
            "Navigation",
            24.0,
            36.0,
            34.0,
            WHITE,
        );

        draw_text(
            "Wheel=zoom | Right drag=pan | R=reset camera | 1=start 2=goal | Enter=route | N=nodes",
            24.0,
            68.0,
            22.0,
            Color::from_rgba(180, 190, 205, 255),
        );
    }

    fn draw_side_panel(&self) {
        let x = SCREEN_WIDTH - SIDE_PANEL_WIDTH;
        let y = TOP_BAR_HEIGHT;
        let w = SIDE_PANEL_WIDTH;
        let h = SCREEN_HEIGHT - TOP_BAR_HEIGHT;

        draw_panel(x, y, w, h);

        let left = SCREEN_WIDTH - SIDE_PANEL_WIDTH + 18.0;

        if !self.is_loading {
            let buttons = self.buttons();

            buttons.mode_fastest.draw(matches!(self.options.mode, RouteMode::Fastest));
            buttons.mode_shortest.draw(matches!(self.options.mode, RouteMode::Shortest));
            buttons.select_start.draw(self.click_mode == ClickMode::SelectStart);
            buttons.select_goal.draw(self.click_mode == ClickMode::SelectGoal);
            buttons.toggle_tolls.draw(self.options.avoid_tolls);
            buttons.toggle_motorway.draw(self.options.avoid_motorways);
            buttons.toggle_traffic.draw(self.options.traffic_enabled);
            buttons.route.draw(false);
        }

        let mut y_text = if self.is_loading {
            TOP_BAR_HEIGHT + 40.0
        } else {
            TOP_BAR_HEIGHT + 430.0
        };

        draw_text("Info", left, y_text, 28.0, WHITE);
        y_text += 30.0;

        self.info_line(left, y_text, "Status", &self.status_text);
        y_text += 28.0;
        self.info_line(left, y_text, "Nodes", &self.graph.nodes.len().to_string());
        y_text += 28.0;
        self.info_line(left, y_text, "Edges", &self.graph.edges.len().to_string());
        y_text += 28.0;
        self.info_line(left, y_text, "Draw Nodes", if self.draw_nodes { "On" } else { "Off" });
        y_text += 28.0;
        self.info_line(left, y_text, "Zoom", &format!("{:.2}x", self.zoom));
        y_text += 28.0;

        let start_name = self
            .start_node
            .map(|n| self.graph.nodes[n].name.as_str())
            .unwrap_or("-");
        let goal_name = self
            .goal_node
            .map(|n| self.graph.nodes[n].name.as_str())
            .unwrap_or("-");

        self.info_line(left, y_text, "Start", start_name);
        y_text += 28.0;
        self.info_line(left, y_text, "Goal", goal_name);
        y_text += 28.0;

        if let Some(route) = &self.route {
            self.info_line(left, y_text, "Found", if route.found { "Yes" } else { "No" });
            y_text += 28.0;
            self.info_line(
                left,
                y_text,
                "Distance",
                &format!("{:.2} km", route.total_distance_m / 1000.0),
            );
            y_text += 28.0;
            self.info_line(
                left,
                y_text,
                "Duration",
                &format!("{:.1} min", route.total_duration_sec / 60.0),
            );
            y_text += 28.0;
            self.info_line(left, y_text, "Visited", &route.visited_nodes.to_string());
        }
    }

    fn info_line(&self, x: f32, y: f32, key: &str, value: &str) {
        draw_text(key, x, y, 21.0, Color::from_rgba(155, 165, 180, 255));
        draw_text(value, x + 120.0, y, 21.0, Color::from_rgba(240, 244, 252, 255));
    }

    fn draw_map(&self) {
        let map_rect = self.map_rect();

        draw_rectangle(
            map_rect.x,
            map_rect.y,
            map_rect.w,
            map_rect.h,
            Color::from_rgba(9, 12, 18, 255),
        );

        if self.is_loading {
            let txt = "Loading map...";
            let m = measure_text(txt, None, 40, 1.0);
            draw_text(
                txt,
                map_rect.x + (map_rect.w - m.width) * 0.5,
                map_rect.y + map_rect.h * 0.5,
                40.0,
                WHITE,
            );
            return;
        }

        if self.graph.nodes.is_empty() || self.bounds.is_none() {
            let txt = "No graph loaded";
            let m = measure_text(txt, None, 40, 1.0);
            draw_text(
                txt,
                map_rect.x + (map_rect.w - m.width) * 0.5,
                map_rect.y + map_rect.h * 0.5,
                40.0,
                WHITE,
            );
            return;
        }

        for edge in &self.graph.edges {
            let a = self.project(self.graph.nodes[edge.from].point);
            let b = self.project(self.graph.nodes[edge.to].point);

            if !self.line_intersects_rect(a, b, map_rect) {
                continue;
            }

            let thickness = (edge.road_type.width() * self.zoom.sqrt()).clamp(1.0, 10.0);
            draw_line(a.x, a.y, b.x, b.y, thickness, edge.road_type.color());
        }

        if let Some(route) = &self.route {
            for edge_id in &route.edge_path {
                let edge = &self.graph.edges[*edge_id];
                let a = self.project(self.graph.nodes[edge.from].point);
                let b = self.project(self.graph.nodes[edge.to].point);

                if !self.line_intersects_rect(a, b, map_rect) {
                    continue;
                }

                draw_line(
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    (6.5 * self.zoom.sqrt()).clamp(2.0, 14.0),
                    Color::from_rgba(0, 255, 153, 255),
                );
            }
        }

        if self.draw_nodes && self.zoom >= 1.5 {
            for node in &self.graph.nodes {
                let p = self.project(node.point);

                if !map_rect.contains(vec2(p.x, p.y)) {
                    continue;
                }

                let mut radius = (1.5 * self.zoom.sqrt()).clamp(1.0, 6.0);
                let mut color = Color::from_rgba(210, 216, 226, 255);

                if Some(node.id) == self.hovered_node {
                    radius = 5.0;
                    color = Color::from_rgba(255, 244, 120, 255);
                }
                if Some(node.id) == self.start_node {
                    radius = 7.0;
                    color = Color::from_rgba(40, 230, 90, 255);
                }
                if Some(node.id) == self.goal_node {
                    radius = 7.0;
                    color = Color::from_rgba(255, 78, 78, 255);
                }

                draw_circle(p.x, p.y, radius, color);
            }
        } else {
            if let Some(start) = self.start_node {
                let p = self.project(self.graph.nodes[start].point);
                if map_rect.contains(vec2(p.x, p.y)) {
                    draw_circle(p.x, p.y, 8.0, Color::from_rgba(40, 230, 90, 255));
                }
            }

            if let Some(goal) = self.goal_node {
                let p = self.project(self.graph.nodes[goal].point);
                if map_rect.contains(vec2(p.x, p.y)) {
                    draw_circle(p.x, p.y, 8.0, Color::from_rgba(255, 78, 78, 255));
                }
            }

            if let Some(hovered) = self.hovered_node {
                let p = self.project(self.graph.nodes[hovered].point);
                if map_rect.contains(vec2(p.x, p.y)) {
                    draw_circle(p.x, p.y, 5.0, Color::from_rgba(255, 244, 120, 255));
                }
            }
        }
    }

    fn buttons(&self) -> Buttons {
        let x = SCREEN_WIDTH - SIDE_PANEL_WIDTH + 18.0;
        let y = TOP_BAR_HEIGHT + 20.0;
        let w = SIDE_PANEL_WIDTH - 36.0;
        let h = 42.0;

        Buttons {
            mode_fastest: Button::new(x, y, w, h, "Fastest"),
            mode_shortest: Button::new(x, y + 50.0, w, h, "Shortest"),
            select_start: Button::new(x, y + 110.0, w, h, "Click: Start"),
            select_goal: Button::new(x, y + 160.0, w, h, "Click: Goal"),
            toggle_tolls: Button::new(x, y + 220.0, w, h, "Avoid Tolls"),
            toggle_motorway: Button::new(x, y + 270.0, w, h, "Avoid Motorways"),
            toggle_traffic: Button::new(x, y + 320.0, w, h, "Traffic"),
            route: Button::new(x, y + 370.0, w, h, "Route Now"),
        }
    }

    fn map_rect(&self) -> Rect {
        Rect::new(
            MAP_PADDING,
            TOP_BAR_HEIGHT + MAP_PADDING,
            SCREEN_WIDTH - SIDE_PANEL_WIDTH - MAP_PADDING * 2.0,
            SCREEN_HEIGHT - TOP_BAR_HEIGHT - MAP_PADDING * 2.0,
        )
    }

    fn project(&self, geo: GeoPoint) -> ScreenPoint {
        let rect = self.map_rect();
        let bounds = self.bounds.unwrap();

        let nx = ((geo.lon - bounds.min_lon) / bounds.width_lon()) as f32;
        let ny = ((geo.lat - bounds.min_lat) / bounds.height_lat()) as f32;

        let base_x = rect.x + nx * rect.w;
        let base_y = rect.y + rect.h - ny * rect.h;

        let cx = rect.x + rect.w * 0.5;
        let cy = rect.y + rect.h * 0.5;

        let x = cx + (base_x - cx) * self.zoom + self.pan_x;
        let y = cy + (base_y - cy) * self.zoom + self.pan_y;

        ScreenPoint { x, y }
    }

    fn find_nearest_node_to_mouse(&self) -> Option<NodeId> {
        let (mx, my) = mouse_position();
        if !self.is_inside_map(mx, my) {
            return None;
        }

        let mut best: Option<(NodeId, f32)> = None;

        for node in &self.graph.nodes {
            let p = self.project(node.point);
            let dx = p.x - mx;
            let dy = p.y - my;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= 18.0 {
                match best {
                    Some((_, best_dist)) if dist >= best_dist => {}
                    _ => best = Some((node.id, dist)),
                }
            }
        }

        best.map(|v| v.0)
    }

    fn is_inside_map(&self, x: f32, y: f32) -> bool {
        self.map_rect().contains(vec2(x, y))
    }

    fn line_intersects_rect(&self, a: ScreenPoint, b: ScreenPoint, rect: Rect) -> bool {
        if rect.contains(vec2(a.x, a.y)) || rect.contains(vec2(b.x, b.y)) {
            return true;
        }

        let min_x = a.x.min(b.x);
        let max_x = a.x.max(b.x);
        let min_y = a.y.min(b.y);
        let max_y = a.y.max(b.y);

        !(max_x < rect.x
            || min_x > rect.x + rect.w
            || max_y < rect.y
            || min_y > rect.y + rect.h)
    }
}

struct Buttons {
    mode_fastest: Button,
    mode_shortest: Button,
    select_start: Button,
    select_goal: Button,
    toggle_tolls: Button,
    toggle_motorway: Button,
    toggle_traffic: Button,
    route: Button,
}