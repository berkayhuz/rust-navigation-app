use std::cmp::Ordering;

pub const SCREEN_WIDTH: f32 = 1400.0;
pub const SCREEN_HEIGHT: f32 = 900.0;

pub const TOP_BAR_HEIGHT: f32 = 92.0;
pub const SIDE_PANEL_WIDTH: f32 = 360.0;
pub const MAP_PADDING: f32 = 20.0;

pub type NodeId = usize;
pub type EdgeId = usize;

#[derive(Debug, Clone, Copy)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

impl GeoBounds {
    pub fn width_lon(&self) -> f64 {
        (self.max_lon - self.min_lon).max(0.00001)
    }

    pub fn height_lat(&self) -> f64 {
        (self.max_lat - self.min_lat).max(0.00001)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoadType {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Residential,
    Service,
    Unclassified,
}

impl RoadType {
    pub fn penalty_shortest_factor(&self) -> f64 {
        match self {
            RoadType::Motorway => 1.00,
            RoadType::Trunk => 1.00,
            RoadType::Primary => 1.00,
            RoadType::Secondary => 1.00,
            RoadType::Tertiary => 1.00,
            RoadType::Residential => 1.00,
            RoadType::Service => 1.08,
            RoadType::Unclassified => 1.02,
        }
    }

    pub fn default_speed_kmh(&self) -> f64 {
        match self {
            RoadType::Motorway => 110.0,
            RoadType::Trunk => 90.0,
            RoadType::Primary => 70.0,
            RoadType::Secondary => 55.0,
            RoadType::Tertiary => 45.0,
            RoadType::Residential => 30.0,
            RoadType::Service => 20.0,
            RoadType::Unclassified => 35.0,
        }
    }

    pub fn color(&self) -> macroquad::prelude::Color {
        use macroquad::prelude::*;
        match self {
            RoadType::Motorway => Color::from_rgba(245, 166, 35, 255),
            RoadType::Trunk => Color::from_rgba(255, 186, 87, 255),
            RoadType::Primary => Color::from_rgba(255, 214, 102, 255),
            RoadType::Secondary => Color::from_rgba(114, 182, 255, 255),
            RoadType::Tertiary => Color::from_rgba(144, 200, 255, 255),
            RoadType::Residential => Color::from_rgba(195, 203, 217, 255),
            RoadType::Service => Color::from_rgba(130, 139, 153, 255),
            RoadType::Unclassified => Color::from_rgba(160, 170, 180, 255),
        }
    }

    pub fn width(&self) -> f32 {
        match self {
            RoadType::Motorway => 5.5,
            RoadType::Trunk => 5.0,
            RoadType::Primary => 4.3,
            RoadType::Secondary => 3.6,
            RoadType::Tertiary => 3.0,
            RoadType::Residential => 2.2,
            RoadType::Service => 1.8,
            RoadType::Unclassified => 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoadNode {
    pub id: NodeId,
    pub point: GeoPoint,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct RoadEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub distance_m: f64,
    pub speed_kmh: f64,
    pub road_type: RoadType,
    pub is_blocked: bool,
    pub has_toll: bool,
    pub traffic_factor: f64,
    pub name: String,
}

#[derive(Debug, Default, Clone)]
pub struct RoadGraph {
    pub nodes: Vec<RoadNode>,
    pub edges: Vec<RoadEdge>,
    pub adjacency: Vec<Vec<EdgeId>>,
}

#[derive(Debug, Clone, Copy)]
pub enum RouteMode {
    Fastest,
    Shortest,
}

impl RouteMode {
    pub fn name(&self) -> &'static str {
        match self {
            RouteMode::Fastest => "Fastest",
            RouteMode::Shortest => "Shortest",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoutingOptions {
    pub mode: RouteMode,
    pub avoid_tolls: bool,
    pub avoid_motorways: bool,
    pub traffic_enabled: bool,
    pub turn_penalty_sec: f64,
}

impl Default for RoutingOptions {
    fn default() -> Self {
        Self {
            mode: RouteMode::Fastest,
            avoid_tolls: false,
            avoid_motorways: false,
            traffic_enabled: true,
            turn_penalty_sec: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub found: bool,
    pub total_cost: f64,
    pub total_distance_m: f64,
    pub total_duration_sec: f64,
    pub node_path: Vec<NodeId>,
    pub edge_path: Vec<EdgeId>,
    pub visited_nodes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct QueueState {
    pub node_id: NodeId,
    pub f_score: f64,
    pub g_score: f64,
}

impl PartialEq for QueueState {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.f_score.to_bits() == other.f_score.to_bits()
            && self.g_score.to_bits() == other.g_score.to_bits()
    }
}

impl Eq for QueueState {}

impl PartialOrd for QueueState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                other
                    .g_score
                    .partial_cmp(&self.g_score)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickMode {
    SelectStart,
    SelectGoal,
}

impl ClickMode {
    pub fn name(&self) -> &'static str {
        match self {
            ClickMode::SelectStart => "Select Start",
            ClickMode::SelectGoal => "Select Goal",
        }
    }
}