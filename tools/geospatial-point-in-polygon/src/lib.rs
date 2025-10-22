//! Geospatial Point-in-Polygon Tool
//!
//! Check if a GPS point is inside a polygon using ray casting algorithm.
//! Useful for geofencing and zone detection.

mod bindings {
    wit_bindgen::generate!({
        world: "geospatial-point-in-polygon",
        generate_all,
    });
}

use bindings::exports::wasmcp::protocol::tools::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasi::io::streams::OutputStream;

struct GeospatialPointInPolygon;

const EPSILON: f64 = 1e-10;

impl Guest for GeospatialPointInPolygon {
    fn list_tools(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListToolsRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Result<ListToolsResult, ErrorCode> {
        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "point_in_polygon".to_string(),
                input_schema: r#"{
                    "type": "object",
                    "properties": {
                        "point": {
                            "type": "object",
                            "properties": {
                                "lat": {"type": "number", "description": "Point latitude"},
                                "lon": {"type": "number", "description": "Point longitude"}
                            },
                            "required": ["lat", "lon"]
                        },
                        "polygon": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "lat": {"type": "number"},
                                    "lon": {"type": "number"}
                                },
                                "required": ["lat", "lon"]
                            },
                            "minItems": 3,
                            "description": "Polygon vertices (at least 3 points)"
                        }
                    },
                    "required": ["point", "polygon"]
                }"#
                .to_string(),
                options: Some(ToolOptions {
                    meta: None,
                    annotations: None,
                    description: Some(
                        "Check if a GPS point is inside a polygon using ray casting algorithm. \
                         Returns whether point is inside, on boundary, and algorithm used. Perfect for geofencing."
                            .to_string(),
                    ),
                    output_schema: None,
                    title: Some("Point in Polygon Check".to_string()),
                }),
            }],
            next_cursor: None,
            meta: None,
        })
    }

    fn call_tool(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        request: CallToolRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Option<CallToolResult> {
        match request.name.as_str() {
            "point_in_polygon" => Some(execute_point_in_polygon(&request.arguments)),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Point {
    lat: f64,
    lon: f64,
}

fn execute_point_in_polygon(arguments: &Option<String>) -> CallToolResult {
    let (point, polygon) = match parse_point_in_polygon_args(arguments) {
        Ok(data) => data,
        Err(msg) => return error_result(msg),
    };

    // Validate polygon has at least 3 vertices
    if polygon.len() < 3 {
        return error_result("Polygon must have at least 3 vertices".to_string());
    }

    // Validate coordinates
    if let Err(msg) = validate_point(&point) {
        return error_result(msg);
    }

    for (i, p) in polygon.iter().enumerate() {
        if let Err(msg) = validate_point(p) {
            return error_result(format!("Polygon vertex {}: {}", i, msg));
        }
    }

    // Check if on boundary
    let on_boundary = is_on_boundary(&point, &polygon);

    // Check if inside using ray casting
    let is_inside = ray_casting_algorithm(&point, &polygon);

    // Format result
    let result = serde_json::json!({
        "is_inside": is_inside,
        "on_boundary": on_boundary,
        "algorithm_used": "ray_casting"
    });

    success_result(result.to_string())
}

fn ray_casting_algorithm(point: &Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let x = point.lon;
    let y = point.lat;
    let mut inside = false;
    let n = polygon.len();

    let mut j = n - 1;
    for i in 0..n {
        let xi = polygon[i].lon;
        let yi = polygon[i].lat;
        let xj = polygon[j].lon;
        let yj = polygon[j].lat;

        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }

    inside
}

fn is_on_boundary(point: &Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let n = polygon.len();

    for i in 0..n {
        let j = (i + 1) % n;
        if is_point_on_segment(point, &polygon[i], &polygon[j]) {
            return true;
        }
    }

    false
}

fn is_point_on_segment(point: &Point, seg_start: &Point, seg_end: &Point) -> bool {
    let cross_product = (point.lat - seg_start.lat) * (seg_end.lon - seg_start.lon)
        - (point.lon - seg_start.lon) * (seg_end.lat - seg_start.lat);

    if cross_product.abs() > EPSILON {
        return false;
    }

    let dot_product = (point.lon - seg_start.lon) * (seg_end.lon - seg_start.lon)
        + (point.lat - seg_start.lat) * (seg_end.lat - seg_start.lat);

    let squared_length = (seg_end.lon - seg_start.lon) * (seg_end.lon - seg_start.lon)
        + (seg_end.lat - seg_start.lat) * (seg_end.lat - seg_start.lat);

    dot_product >= 0.0 && dot_product <= squared_length
}

fn validate_point(point: &Point) -> Result<(), String> {
    if point.lat.is_nan() || point.lat.is_infinite() {
        return Err("Latitude cannot be NaN or infinite".to_string());
    }
    if point.lon.is_nan() || point.lon.is_infinite() {
        return Err("Longitude cannot be NaN or infinite".to_string());
    }
    if point.lat < -90.0 || point.lat > 90.0 {
        return Err(format!(
            "Invalid latitude: {}. Must be between -90 and 90",
            point.lat
        ));
    }
    if point.lon < -180.0 || point.lon > 180.0 {
        return Err(format!(
            "Invalid longitude: {}. Must be between -180 and 180",
            point.lon
        ));
    }
    Ok(())
}

fn parse_point_in_polygon_args(
    arguments: &Option<String>,
) -> Result<(Point, Vec<Point>), String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(args_str).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    // Parse point
    let point_obj = json
        .get("point")
        .ok_or_else(|| "Missing 'point' parameter".to_string())?;

    let point_lat = point_obj
        .get("lat")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid 'point.lat'".to_string())?;

    let point_lon = point_obj
        .get("lon")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid 'point.lon'".to_string())?;

    let point = Point {
        lat: point_lat,
        lon: point_lon,
    };

    // Parse polygon
    let polygon_arr = json
        .get("polygon")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing or invalid 'polygon' parameter".to_string())?;

    let mut polygon = Vec::new();
    for (i, vertex) in polygon_arr.iter().enumerate() {
        let lat = vertex
            .get("lat")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Missing or invalid 'polygon[{}].lat'", i))?;

        let lon = vertex
            .get("lon")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Missing or invalid 'polygon[{}].lon'", i))?;

        polygon.push(Point { lat, lon });
    }

    Ok((point, polygon))
}

fn success_result(result: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(result),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }
}

fn error_result(message: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(message),
            options: None,
        })],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

bindings::export!(GeospatialPointInPolygon with_types_in bindings);
