//! Geospatial Bearing Tool
//!
//! Calculate bearing/heading between GPS coordinates.
//! Returns bearing in degrees, radians, and compass direction.

mod bindings {
    wit_bindgen::generate!({
        world: "geospatial-bearing",
        generate_all,
    });
}

use bindings::exports::wasmcp::protocol::tools::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasi::io::streams::OutputStream;
use std::f64::consts::PI;

struct GeospatialBearing;

impl Guest for GeospatialBearing {
    fn list_tools(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListToolsRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Result<ListToolsResult, ErrorCode> {
        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "bearing".to_string(),
                input_schema: r#"{
                    "type": "object",
                    "properties": {
                        "lat1": {"type": "number", "description": "Latitude of start point (-90 to 90)"},
                        "lon1": {"type": "number", "description": "Longitude of start point (-180 to 180)"},
                        "lat2": {"type": "number", "description": "Latitude of end point (-90 to 90)"},
                        "lon2": {"type": "number", "description": "Longitude of end point (-180 to 180)"}
                    },
                    "required": ["lat1", "lon1", "lat2", "lon2"]
                }"#
                .to_string(),
                options: Some(ToolOptions {
                    meta: None,
                    annotations: None,
                    description: Some(
                        "Calculate bearing/heading from one GPS coordinate to another. \
                         Returns bearing in degrees (0-360), radians, and compass direction (N, NE, E, etc.)."
                            .to_string(),
                    ),
                    output_schema: None,
                    title: Some("GPS Bearing Calculator".to_string()),
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
            "bearing" => Some(execute_bearing(&request.arguments)),
            _ => None,
        }
    }
}

fn execute_bearing(arguments: &Option<String>) -> CallToolResult {
    let (lat1, lon1, lat2, lon2) = match parse_bearing_args(arguments) {
        Ok(coords) => coords,
        Err(msg) => return error_result(msg),
    };

    // Validate coordinates
    if let Err(msg) = validate_coordinates(lat1, lon1, lat2, lon2) {
        return error_result(msg);
    }

    // Calculate bearing
    let bearing_deg = calculate_bearing(lat1, lon1, lat2, lon2);
    let bearing_rad = bearing_deg * PI / 180.0;
    let compass = degrees_to_compass(bearing_deg);

    // Format result
    let result = serde_json::json!({
        "bearing_degrees": bearing_deg,
        "bearing_radians": bearing_rad,
        "compass_direction": compass
    });

    success_result(result.to_string())
}

fn calculate_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lon = (lon2 - lon1) * PI / 180.0;

    let y = delta_lon.sin() * lat2_rad.cos();
    let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

    let bearing_rad = y.atan2(x);

    (bearing_rad * 180.0 / PI + 360.0) % 360.0
}

fn degrees_to_compass(degrees: f64) -> String {
    let directions = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];

    let index = ((degrees + 11.25) / 22.5) as usize % 16;
    directions[index].to_string()
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

fn parse_bearing_args(arguments: &Option<String>) -> Result<(f64, f64, f64, f64), String> {
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

bindings::export!(GeospatialBearing with_types_in bindings);
