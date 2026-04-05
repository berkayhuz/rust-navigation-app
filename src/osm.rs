use crate::graph::haversine_meters;
use crate::types::*;
use osmpbfreader::{OsmObj, OsmPbfReader, Tags, Way};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

pub fn load_graph_from_pbf(path: impl AsRef<Path>) -> Result<RoadGraph, String> {
    let file = File::open(path.as_ref())
        .map_err(|e| format!("PBF dosyası açılamadı: {}", e))?;

    let mut pbf = OsmPbfReader::new(file);

    let objects = pbf
        .get_objs_and_deps(|obj| match obj {
            OsmObj::Way(way) => is_routable_way(way),
            _ => false,
        })
        .map_err(|e| format!("PBF okunamadı: {}", e))?;

    let mut way_list: Vec<Way> = Vec::new();
    let mut used_osm_nodes = HashSet::new();

    for obj in objects.values() {
        if let OsmObj::Way(way) = obj {
            if !is_routable_way(way) {
                continue;
            }

            for node_id in &way.nodes {
                used_osm_nodes.insert(*node_id);
            }

            way_list.push(way.clone());
        }
    }

    let mut graph = RoadGraph::new();
    let mut osm_to_graph = HashMap::new();

    for obj in objects.values() {
        if let OsmObj::Node(node) = obj {
            if !used_osm_nodes.contains(&node.id) {
                continue;
            }

            let lat = node.lat();
            let lon = node.lon();

            let graph_id = graph.add_node(
                GeoPoint { lat, lon },
                format!("osm:{}", node.id.0),
            );

            osm_to_graph.insert(node.id, graph_id);
        }
    }

    for way in &way_list {
        let road_type = parse_road_type(&way.tags);
        let speed_kmh = parse_speed(&way.tags).unwrap_or_else(|| road_type.default_speed_kmh());
        let has_toll = has_toll(&way.tags);
        let name = way
            .tags
            .get("name")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("way:{}", way.id.0));

        let oneway = is_oneway(&way.tags, road_type);

        for pair in way.nodes.windows(2) {
            let from_osm = pair[0];
            let to_osm = pair[1];

            let Some(&from) = osm_to_graph.get(&from_osm) else {
                continue;
            };
            let Some(&to) = osm_to_graph.get(&to_osm) else {
                continue;
            };

            let from_point = graph.nodes[from].point;
            let to_point = graph.nodes[to].point;
            let dist = haversine_meters(from_point, to_point);

            graph.add_directed_edge(
                from,
                to,
                dist,
                speed_kmh,
                road_type,
                has_toll,
                1.0,
                name.clone(),
            );

            if !oneway {
                graph.add_directed_edge(
                    to,
                    from,
                    dist,
                    speed_kmh,
                    road_type,
                    has_toll,
                    1.0,
                    name.clone(),
                );
            }
        }
    }

    Ok(graph)
}

fn is_routable_way(way: &Way) -> bool {
    if way.nodes.len() < 2 {
        return false;
    }

    let Some(highway) = way.tags.get("highway") else {
        return false;
    };

    matches!(
        highway.as_str(),
        "motorway"
            | "motorway_link"
            | "trunk"
            | "trunk_link"
            | "primary"
            | "primary_link"
            | "secondary"
            | "secondary_link"
            | "tertiary"
            | "tertiary_link"
            | "residential"
            | "living_street"
            | "service"
            | "unclassified"
    )
}

fn parse_road_type(tags: &Tags) -> RoadType {
    match tags.get("highway").map(|s| s.as_str()) {
        Some("motorway") | Some("motorway_link") => RoadType::Motorway,
        Some("trunk") | Some("trunk_link") => RoadType::Trunk,
        Some("primary") | Some("primary_link") => RoadType::Primary,
        Some("secondary") | Some("secondary_link") => RoadType::Secondary,
        Some("tertiary") | Some("tertiary_link") => RoadType::Tertiary,
        Some("service") => RoadType::Service,
        Some("residential") | Some("living_street") => RoadType::Residential,
        _ => RoadType::Unclassified,
    }
}

fn parse_speed(tags: &Tags) -> Option<f64> {
    let raw = tags.get("maxspeed")?;
    let normalized = raw.trim().to_lowercase();

    if let Some(v) = normalized.strip_suffix(" km/h") {
        return v.trim().parse::<f64>().ok();
    }
    if let Some(v) = normalized.strip_suffix("km/h") {
        return v.trim().parse::<f64>().ok();
    }
    if let Some(v) = normalized.strip_suffix("mph") {
        let mph = v.trim().parse::<f64>().ok()?;
        return Some(mph * 1.60934);
    }

    normalized.parse::<f64>().ok()
}

fn is_oneway(tags: &Tags, road_type: RoadType) -> bool {
    match tags.get("oneway").map(|s| s.as_str()) {
        Some("yes") | Some("1") | Some("true") => true,
        Some("no") | Some("0") | Some("false") => false,
        Some("-1") => true,
        _ => matches!(road_type, RoadType::Motorway),
    }
}

fn has_toll(tags: &Tags) -> bool {
    match tags.get("toll").map(|s| s.as_str()) {
        Some("yes") | Some("true") | Some("1") => true,
        _ => false,
    }
}