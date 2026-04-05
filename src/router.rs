use crate::types::*;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub struct AStarRouter;

impl AStarRouter {
    pub fn find_route(
        graph: &RoadGraph,
        start: NodeId,
        goal: NodeId,
        options: &RoutingOptions,
    ) -> RouteResult {
        let mut open_heap = BinaryHeap::<QueueState>::new();
        let mut closed = HashSet::<NodeId>::new();

        let mut g_score = HashMap::<NodeId, f64>::new();
        let mut parent_node = HashMap::<NodeId, NodeId>::new();
        let mut parent_edge = HashMap::<NodeId, EdgeId>::new();

        g_score.insert(start, 0.0);

        open_heap.push(QueueState {
            node_id: start,
            g_score: 0.0,
            f_score: heuristic_cost(graph, start, goal, options),
        });

        let mut visited_nodes = 0usize;

        while let Some(current) = open_heap.pop() {
            let best_known = *g_score.get(&current.node_id).unwrap_or(&f64::INFINITY);
            if current.g_score > best_known {
                continue;
            }

            if !closed.insert(current.node_id) {
                continue;
            }

            visited_nodes += 1;

            if current.node_id == goal {
                return reconstruct(
                    graph,
                    start,
                    goal,
                    &parent_node,
                    &parent_edge,
                    current.g_score,
                    visited_nodes,
                );
            }

            for edge in graph.outgoing_edges(current.node_id) {
                if !edge_allowed(edge, options) {
                    continue;
                }

                let next = edge.to;
                let edge_cost = edge_cost(edge, options);
                let tentative_g = current.g_score + edge_cost + options.turn_penalty_sec;

                if tentative_g < *g_score.get(&next).unwrap_or(&f64::INFINITY) {
                    g_score.insert(next, tentative_g);
                    parent_node.insert(next, current.node_id);
                    parent_edge.insert(next, edge.id);

                    let h = heuristic_cost(graph, next, goal, options);

                    open_heap.push(QueueState {
                        node_id: next,
                        g_score: tentative_g,
                        f_score: tentative_g + h,
                    });
                }
            }
        }

        RouteResult {
            found: false,
            total_cost: f64::INFINITY,
            total_distance_m: 0.0,
            total_duration_sec: 0.0,
            node_path: vec![],
            edge_path: vec![],
            visited_nodes,
        }
    }
}

fn edge_allowed(edge: &RoadEdge, options: &RoutingOptions) -> bool {
    if edge.is_blocked {
        return false;
    }
    if options.avoid_tolls && edge.has_toll {
        return false;
    }
    if options.avoid_motorways && edge.road_type == RoadType::Motorway {
        return false;
    }
    true
}

fn edge_cost(edge: &RoadEdge, options: &RoutingOptions) -> f64 {
    match options.mode {
        RouteMode::Shortest => edge.distance_m * edge.road_type.penalty_shortest_factor(),
        RouteMode::Fastest => {
            let mps = edge.speed_kmh.max(1.0) * 1000.0 / 3600.0;
            let mut sec = edge.distance_m / mps;
            if options.traffic_enabled {
                sec *= edge.traffic_factor.max(1.0);
            }
            sec
        }
    }
}

fn heuristic_cost(
    graph: &RoadGraph,
    from: NodeId,
    to: NodeId,
    options: &RoutingOptions,
) -> f64 {
    let a = graph.nodes[from].point;
    let b = graph.nodes[to].point;
    let dist_m = haversine_meters(a, b);

    match options.mode {
        RouteMode::Shortest => dist_m,
        RouteMode::Fastest => {
            let optimistic_speed_kmh = 130.0;
            dist_m / (optimistic_speed_kmh * 1000.0 / 3600.0)
        }
    }
}

fn reconstruct(
    graph: &RoadGraph,
    start: NodeId,
    goal: NodeId,
    parent_node: &HashMap<NodeId, NodeId>,
    parent_edge: &HashMap<NodeId, EdgeId>,
    total_cost: f64,
    visited_nodes: usize,
) -> RouteResult {
    let mut node_path = vec![goal];
    let mut edge_path = Vec::<EdgeId>::new();
    let mut total_distance_m = 0.0;
    let mut total_duration_sec = 0.0;

    let mut current = goal;

    while current != start {
        let Some(&prev) = parent_node.get(&current) else {
            break;
        };

        let Some(&edge_id) = parent_edge.get(&current) else {
            break;
        };

        let edge = &graph.edges[edge_id];
        edge_path.push(edge_id);
        node_path.push(prev);

        total_distance_m += edge.distance_m;

        let mps = edge.speed_kmh.max(1.0) * 1000.0 / 3600.0;
        total_duration_sec += (edge.distance_m / mps) * edge.traffic_factor.max(1.0);

        current = prev;
    }

    node_path.reverse();
    edge_path.reverse();

    RouteResult {
        found: true,
        total_cost,
        total_distance_m,
        total_duration_sec,
        node_path,
        edge_path,
        visited_nodes,
    }
}

pub fn haversine_meters(a: GeoPoint, b: GeoPoint) -> f64 {
    let r = 6_371_000.0_f64;

    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let dlat = (b.lat - a.lat).to_radians();
    let dlon = (b.lon - a.lon).to_radians();

    let sin_dlat = (dlat / 2.0).sin();
    let sin_dlon = (dlon / 2.0).sin();

    let aa = sin_dlat * sin_dlat + lat1.cos() * lat2.cos() * sin_dlon * sin_dlon;
    let c = 2.0 * aa.sqrt().atan2((1.0 - aa).sqrt());

    r * c
}