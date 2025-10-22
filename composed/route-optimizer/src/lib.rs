//! Route Optimizer Middleware
//!
//! Analyzes routes between multiple GPS waypoints using distance and bearing calculations.
//! Chains distance and bearing tools to provide comprehensive route analysis.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "route-optimizer",
        generate_all,
    });
}

use bindings::exports::wasmcp::server::handler::Guest;
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasmcp::protocol::server_messages::Context;
use bindings::wasmcp::server::handler as downstream;

struct RouteOptimizer;

impl Guest for RouteOptimizer {
    fn handle_request(
        ctx: Context,
        request: (ClientRequest, RequestId),
        client_stream: Option<&OutputStream>,
    ) -> Result<ServerResponse, ErrorCode> {
        let (req, id) = request;
        match req {
            ClientRequest::ToolsList(list_req) => {
                handle_tools_list(list_req, id, &ctx, client_stream)
            }
            ClientRequest::ToolsCall(ref call_req) => {
                if call_req.name == "analyze_route" {
                    handle_analyze_route(call_req.clone(), id, &ctx, client_stream)
                } else {
                    downstream::handle_request(&ctx, (&req, &id), client_stream)
                }
            }
            _ => downstream::handle_request(&ctx, (&req, &id), client_stream),
        }
    }

    fn handle_notification(ctx: Context, notification: ClientNotification) {
        downstream::handle_notification(&ctx, &notification);
    }

    fn handle_response(ctx: Context, response: Result<(ClientResponse, RequestId), ErrorCode>) {
        downstream::handle_response(&ctx, response);
    }
}

fn handle_tools_list(
    req: ListToolsRequest,
    id: RequestId,
    ctx: &Context,
    client_stream: Option<&OutputStream>,
) -> Result<ServerResponse, ErrorCode> {
    let route_tool = Tool {
        name: "analyze_route".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {
                "waypoints": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "lat": {"type": "number"},
                            "lon": {"type": "number"}
                        },
                        "required": ["lat", "lon"]
                    },
                    "minItems": 2,
                    "description": "Route waypoints (at least 2 points)"
                }
            },
            "required": ["waypoints"]
        }"#
        .to_string(),
        options: Some(ToolOptions {
            meta: None,
            annotations: None,
            description: Some(
                "Analyze a route through multiple GPS waypoints. \
                 Returns total distance, segment distances, and bearings between each waypoint. \
                 Chains distance and bearing calculations for comprehensive route analysis."
                    .to_string(),
            ),
            output_schema: None,
            title: Some("Route Analyzer".to_string()),
        }),
    };

    let downstream_req = ClientRequest::ToolsList(req.clone());
    match downstream::handle_request(ctx, (&downstream_req, &id), client_stream) {
        Ok(ServerResponse::ToolsList(mut downstream_result)) => {
            downstream_result.tools.push(route_tool);
            Ok(ServerResponse::ToolsList(downstream_result))
        }
        Err(ErrorCode::MethodNotFound(_)) => Ok(ServerResponse::ToolsList(ListToolsResult {
            tools: vec![route_tool],
            next_cursor: None,
            meta: None,
        })),
        Err(_) | Ok(_) => Ok(ServerResponse::ToolsList(ListToolsResult {
            tools: vec![route_tool],
            next_cursor: None,
            meta: None,
        })),
    }
}

fn handle_analyze_route(
    request: CallToolRequest,
    id: RequestId,
    ctx: &Context,
    client_stream: Option<&OutputStream>,
) -> Result<ServerResponse, ErrorCode> {
    let waypoints = match parse_waypoints(&request.arguments) {
        Ok(w) => w,
        Err(msg) => return Ok(ServerResponse::ToolsCall(error_result(msg))),
    };

    if waypoints.len() < 2 {
        return Ok(ServerResponse::ToolsCall(error_result(
            "Route must have at least 2 waypoints".to_string(),
        )));
    }

    let mut segments = Vec::new();
    let mut total_distance_km = 0.0;

    for i in 0..waypoints.len() - 1 {
        let from = &waypoints[i];
        let to = &waypoints[i + 1];

        let distance_args = format!(
            r#"{{"lat1": {}, "lon1": {}, "lat2": {}, "lon2": {}}}"#,
            from.0, from.1, to.0, to.1
        );

        let distance_result = match call_downstream_tool(ctx, "distance", &distance_args, &id, client_stream) {
            Ok(r) => r,
            Err(msg) => return Ok(ServerResponse::ToolsCall(error_result(msg))),
        };

        let bearing_result = match call_downstream_tool(ctx, "bearing", &distance_args, &id, client_stream) {
            Ok(r) => r,
            Err(msg) => return Ok(ServerResponse::ToolsCall(error_result(msg))),
        };

        let dist_json: serde_json::Value = serde_json::from_str(&distance_result)
            .unwrap_or_else(|_| serde_json::json!({"distance_km": 0.0}));
        let bearing_json: serde_json::Value = serde_json::from_str(&bearing_result)
            .unwrap_or_else(|_| serde_json::json!({"bearing_degrees": 0.0, "compass_direction": "N"}));

        let segment_distance = dist_json["distance_km"].as_f64().unwrap_or(0.0);
        total_distance_km += segment_distance;

        segments.push(serde_json::json!({
            "from": {"lat": from.0, "lon": from.1},
            "to": {"lat": to.0, "lon": to.1},
            "distance_km": segment_distance,
            "distance_miles": dist_json["distance_miles"],
            "bearing_degrees": bearing_json["bearing_degrees"],
            "compass_direction": bearing_json["compass_direction"]
        }));
    }

    let result = serde_json::json!({
        "total_waypoints": waypoints.len(),
        "total_distance_km": total_distance_km,
        "total_distance_miles": total_distance_km * 0.621371,
        "segments": segments
    });

    Ok(ServerResponse::ToolsCall(CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(result.to_string()),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }))
}

fn call_downstream_tool(
    ctx: &Context,
    tool_name: &str,
    arguments: &str,
    request_id: &RequestId,
    client_stream: Option<&OutputStream>,
) -> Result<String, String> {
    let tool_request = CallToolRequest {
        name: tool_name.to_string(),
        arguments: Some(arguments.to_string()),
    };

    let downstream_req = ClientRequest::ToolsCall(tool_request);

    match downstream::handle_request(ctx, (&downstream_req, request_id), client_stream) {
        Ok(ServerResponse::ToolsCall(result)) => {
            if result.is_error == Some(true) {
                return Err(format!("Tool '{}' returned an error", tool_name));
            }
            if let Some(ContentBlock::Text(text)) = result.content.first() {
                if let TextData::Text(content) = &text.text {
                    return Ok(content.clone());
                }
            }
            Ok("{}".to_string())
        }
        Ok(_) => Err(format!("Unexpected response type from '{}'", tool_name)),
        Err(ErrorCode::MethodNotFound(_)) => Err(format!(
            "Tool '{}' not found. Ensure geospatial tools come AFTER route-optimizer in the pipeline.",
            tool_name
        )),
        Err(e) => Err(format!("Error calling '{}': {:?}", tool_name, e)),
    }
}

fn parse_waypoints(arguments: &Option<String>) -> Result<Vec<(f64, f64)>, String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(args_str).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let waypoints_arr = json
        .get("waypoints")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing or invalid 'waypoints' parameter".to_string())?;

    let mut waypoints = Vec::new();
    for (i, wp) in waypoints_arr.iter().enumerate() {
        let lat = wp
            .get("lat")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Missing or invalid 'waypoints[{}].lat'", i))?;

        let lon = wp
            .get("lon")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Missing or invalid 'waypoints[{}].lon'", i))?;

        waypoints.push((lat, lon));
    }

    Ok(waypoints)
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

bindings::export!(RouteOptimizer with_types_in bindings);
