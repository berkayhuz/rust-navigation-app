use crate::types::*;

impl RoadGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, point: GeoPoint, name: impl Into<String>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(RoadNode {
            id,
            point,
            name: name.into(),
        });
        self.adjacency.push(Vec::new());
        id
    }

    pub fn add_directed_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        distance_m: f64,
        speed_kmh: f64,
        road_type: RoadType,
        has_toll: bool,
        traffic_factor: f64,
        name: impl Into<String>,
    ) -> EdgeId {
        let id = self.edges.len();
        self.edges.push(RoadEdge {
            id,
            from,
            to,
            distance_m,
            speed_kmh,
            road_type,
            is_blocked: false,
            has_toll,
            traffic_factor,
            name: name.into(),
        });
        self.adjacency[from].push(id);
        id
    }

    pub fn outgoing_edges(&self, node_id: NodeId) -> impl Iterator<Item = &RoadEdge> {
        self.adjacency[node_id]
            .iter()
            .map(|edge_id| &self.edges[*edge_id])
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