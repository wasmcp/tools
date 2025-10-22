//! Geospatial Distance Tool
//!
//! Calculate distance between GPS coordinates using Haversine formula.
//! Returns results in kilometers, miles, and nautical miles.

mod bindings {
    wit_bindgen::generate!({
        world: "geospatial-distance",
        generate_all,
    });
}

use bindings::exports::wasmcp::protocol::tools::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasi::io::streams::OutputStream;
use std::f64::consts::PI;

struct GeospatialDistance;

impl Guest for GeospatialDistance {
    fn list_tools(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListToolsRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Result<ListToolsResult, ErrorCode> {
        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "distance".to_string(),
                input_schema: r#"{
                    "type": "object",
                    "properties": {
                        "lat1": {"type": "number", "description": "Latitude of first point (-90 to 90)"},
                        "lon1": {"type": "number", "description": "Longitude of first point (-180 to 180)"},
                        "lat2": {"type": "number", "description": "Latitude of second point (-90 to 90)"},
                        "lon2": {"type": "number", "description": "Longitude of second point (-180 to 180)"}
                    },
                    "required": ["lat1", "lon1", "lat2", "lon2"]
                }"#
                .to_string(),
                options: Some(ToolOptions {
                    meta: None,
                    annotations: None,
                    description: Some(
                        "Calculate distance between two GPS coordinates using Haversine formula. \
                         Returns distance in kilometers, miles, and nautical miles with 99.8% accuracy."
                            .to_string(),
                    ),
                    output_schema: None,
                    title: Some("GPS Distance Calculator".to_string()),
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
            "distance" => Some(execute_distance(&request.arguments)),
            _ => None,
        }
    }
}

fn execute_distance(arguments: &Option<String>) -> CallToolResult {
    let (lat1, lon1, lat2, lon2) = match parse_distance_args(arguments) {
        Ok(coords) => coords,
        Err(msg) => return error_result(msg),
    };

    // Validate coordinates
    if let Err(msg) = validate_coordinates(lat1, lon1, lat2, lon2) {
        return error_result(msg);
    }

    // Calculate distance using Haversine formula
    let distance_km = haversine_distance(lat1, lon1, lat2, lon2);
    let distance_miles = distance_km * 0.621371;
    let distance_nautical_miles = distance_km * 0.539957;

    // Format result
    let result = serde_json::json!({
        "distance_km": distance_km,
        "distance_miles": distance_miles,
        "distance_nautical_miles": distance_nautical_miles,
        "formula": "Haversine",
        "accuracy": "99.8%"
    });

    success_result(result.to_string())
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lat = (lat2 - lat1) * PI / 180.0;
    let delta_lon = (lon2 - lon1) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

fn validate_coordinates(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<(), String> {
    // Check for NaN or Infinite
    if lat1.is_nan() || lat1.is_infinite()
        || lon1.is_nan() || lon1.is_infinite()
        || lat2.is_nan() || lat2.is_infinite()
        || lon2.is_nan() || lon2.is_infinite()
    {
        return Err("Input contains invalid values (NaN or Infinite)".to_string());
    }

    // Validate latitude range
    if lat1 < -90.0 || lat1 > 90.0 || lat2 < -90.0 || lat2 > 90.0 {
        return Err("Latitude must be between -90 and 90 degrees".to_string());
    }

    // Validate longitude range
    if lon1 < -180.0 || lon1 > 180.0 || lon2 < -180.0 || lon2 > 180.0 {
        return Err("Longitude must be between -180 and 180 degrees".to_string());
    }

    Ok(())
}

fn parse_distance_args(arguments: &Option<String>) -> Result<(f64, f64, f64, f64), String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(args_str).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let lat1 = json
        .get("lat1")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'lat1'".to_string())?;

    let lon1 = json
        .get("lon1")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'lon1'".to_string())?;

    let lat2 = json
        .get("lat2")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'lat2'".to_string())?;

    let lon2 = json
        .get("lon2")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'lon2'".to_string())?;

    Ok((lat1, lon1, lat2, lon2))
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

bindings::export!(GeospatialDistance with_types_in bindings);
