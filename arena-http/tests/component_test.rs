use arena::dependency::RunnableDependency;
use arena_http::{
    created, delete_requested_for, get_requested_for, no_content, ok_json, post_requested_for,
    put_requested_for, server_error, status, ActivePlaybook, HttpDependency,
};
use futures::FutureExt;
use serde_json::json;

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

struct TestContext {
    http_dependency: HttpDependency,
    base_url: String,
    client: reqwest::Client,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        tracing::info!(suite = "crate_component", crate_under_test = "arena_http", phase = "dependency_start_begin", "starting dependency");
        let mut http_dependency = HttpDependency::builder("").build();
        http_dependency.start().await;

        let base_url = http_dependency
            .base_url()
            .ok_or_else(|| "http base_url missing after start()".to_string())?
            .to_string();

        tracing::info!(suite = "crate_component", crate_under_test = "arena_http", phase = "dependency_running", base_url = %base_url, "dependency reachable");
        Ok(Self {
            http_dependency,
            base_url,
            client: reqwest::Client::new(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    async fn fire_get(&self, path: &str) -> Result<reqwest::Response, String> {
        self.client
            .get(self.url(path))
            .send()
            .await
            .map_err(|e| format!("GET {path} failed: {e}"))
    }

    async fn fire_post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response, String> {
        self.client
            .post(self.url(path))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("POST {path} failed: {e}"))
    }

    async fn fire_put(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<reqwest::Response, String> {
        self.client
            .put(self.url(path))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("PUT {path} failed: {e}"))
    }

    async fn fire_delete(&self, path: &str) -> Result<reqwest::Response, String> {
        self.client
            .delete(self.url(path))
            .send()
            .await
            .map_err(|e| format!("DELETE {path} failed: {e}"))
    }

    async fn stop(mut self) {
        self.http_dependency.stop().await;
    }
}

async fn setup_guidance_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/guidance/state")
        .will_return(ok_json(json!({
            "mode": "free-return",
            "attitude_deg": { "roll": 0.2, "pitch": -1.4, "yaw": 0.0 }
        })))
        .run()
        .await
}

async fn setup_ecs_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/ecs/cabin-atmosphere")
        .will_return(ok_json(json!({
            "o2_psi": 4.8,
            "co2_mmhg": 3.1,
            "temp_f": 72
        })))
        .put("/api/ecs/o2-flow")
        .will_return(ok_json(json!({ "o2_flow_lb_hr": 0.96 })))
        .run()
        .await
}

async fn setup_eps_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/eps/fuel-cells")
        .will_return(ok_json(json!({
            "cells": [
                { "id": 1, "voltage": 29.0, "amps": 20.5 },
                { "id": 2, "voltage": 29.1, "amps": 19.8 },
                { "id": 3, "voltage": 28.9, "amps": 20.1 }
            ]
        })))
        .get("/api/eps/bus-voltage")
        .will_return(ok_json(
            json!({ "main_bus_a_v": 29.0, "main_bus_b_v": 29.1 }),
        ))
        .run()
        .await
}

async fn setup_sps_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .post("/api/sps/ignition")
        .will_return(ok_json(json!({
            "burn_duration_s": 5.9,
            "delta_v_fps": 32.4
        })))
        .delete("/api/sps/shutdown")
        .will_return(no_content())
        .run()
        .await
}

async fn setup_rcs_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/rcs/thruster-status")
        .will_return(ok_json(json!({
            "quads": {
                "a": "nominal", "b": "nominal",
                "c": "nominal", "d": "nominal"
            }
        })))
        .post("/api/rcs/attitude-correction")
        .will_return(ok_json(json!({ "correction_applied": true })))
        .run()
        .await
}

async fn setup_comms_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .post("/api/comms/downlink")
        .will_return(ok_json(json!({
            "signal": "s-band",
            "station": "goldstone",
            "strength_dbm": -120
        })))
        .run()
        .await
}

async fn setup_thermal_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/thermal/radiator-status")
        .will_return(ok_json(json!({
            "radiator_1_f": 45,
            "radiator_2_f": 42,
            "coolant_flow_ok": true
        })))
        .run()
        .await
}

async fn setup_docking_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .post("/api/docking/initiate")
        .will_return(ok_json(json!({
            "target": "lunar-module",
            "capture_latches": "engaged"
        })))
        .delete("/api/docking/separate")
        .will_return(no_content())
        .run()
        .await
}

async fn setup_telemetry_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/telemetry/downlink")
        .will_return(ok_json(json!({
            "bit_rate_kbps": 51.2,
            "frames_sent": 14832
        })))
        .run()
        .await
}

async fn setup_cabin_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency
        .playbook()
        .get("/api/cabin/pressure")
        .will_return(ok_json(json!({ "pressure_psi": 5.0, "nominal": true })))
        .put("/api/cabin/pressure")
        .will_return(ok_json(json!({ "pressure_psi": 5.2, "nominal": true })))
        .run()
        .await
}

async fn run_spacecraft_inventory_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx
        .http_dependency
        .playbook()
        .get("/api/spacecraft")
        .will_return(ok_json(json!({
            "spacecraft": [
                { "id": 1, "name": "Eagle Transporter", "class": "freighter" },
                { "id": 2, "name": "Discovery One", "class": "exploration" }
            ]
        })))
        .get("/api/spacecraft/1")
        .will_return(ok_json(json!({
            "id": 1,
            "name": "Eagle Transporter",
            "class": "freighter",
            "crew_capacity": 4
        })))
        .post("/api/spacecraft")
        .will_return(
            created()
                .with_json_body(json!({ "id": 3, "name": "Rocinante", "class": "corvette" }))
                .with_header("Location", "/api/spacecraft/3"),
        )
        .put("/api/spacecraft/1")
        .will_return(ok_json(json!({
            "id": 1,
            "name": "Eagle Transporter",
            "class": "freighter",
            "crew_capacity": 6
        })))
        .delete("/api/spacecraft/2")
        .will_return(no_content())
        .run()
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "spacecraft_inventory", phase = "endpoints_ready", "playbook applied, calling endpoints");

    let resp = ctx.fire_get("/api/spacecraft").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["spacecraft"].as_array().unwrap().len(), 2);

    let resp = ctx.fire_get("/api/spacecraft/1").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["name"], "Eagle Transporter");

    let resp = ctx
        .fire_post(
            "/api/spacecraft",
            json!({ "name": "Rocinante", "class": "corvette" }),
        )
        .await?;
    assert_eq!(resp.status().as_u16(), 201);
    let location = resp
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(location, "/api/spacecraft/3");

    let resp = ctx
        .fire_put("/api/spacecraft/1", json!({ "crew_capacity": 6 }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["crew_capacity"], 6);

    let resp = ctx.fire_delete("/api/spacecraft/2").await?;
    assert_eq!(resp.status().as_u16(), 204);

    playbook
        .verify(1, get_requested_for("/api/spacecraft"))
        .await;
    playbook
        .verify(1, get_requested_for("/api/spacecraft/1"))
        .await;
    playbook
        .verify(1, post_requested_for("/api/spacecraft"))
        .await;
    playbook
        .verify(1, put_requested_for("/api/spacecraft/1"))
        .await;
    playbook
        .verify(1, delete_requested_for("/api/spacecraft/2"))
        .await;

    let post_requests = playbook
        .find_requests(post_requested_for("/api/spacecraft"))
        .await;
    assert_eq!(post_requests.len(), 1);
    assert!(post_requests[0].body.contains("Rocinante"));

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "spacecraft_inventory", phase = "scenario_ok", "all verifications passed");
    Ok(())
}

async fn run_scoped_playbooks_test(ctx: &TestContext) -> Result<(), String> {
    let guidance = setup_guidance_playbook(ctx).await;
    let ecs = setup_ecs_playbook(ctx).await;
    let eps = setup_eps_playbook(ctx).await;
    let sps = setup_sps_playbook(ctx).await;
    let rcs = setup_rcs_playbook(ctx).await;
    let comms = setup_comms_playbook(ctx).await;
    let thermal = setup_thermal_playbook(ctx).await;
    let docking = setup_docking_playbook(ctx).await;
    let telemetry = setup_telemetry_playbook(ctx).await;
    let cabin = setup_cabin_playbook(ctx).await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_playbooks", phase = "requests_begin", "playbooks applied, firing requests");

    for _ in 0..3 {
        let resp = ctx.fire_get("/api/guidance/state").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..2 {
        let resp = ctx.fire_get("/api/ecs/cabin-atmosphere").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    let resp = ctx
        .fire_put("/api/ecs/o2-flow", json!({ "o2_flow_lb_hr": 0.96 }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    for _ in 0..4 {
        let resp = ctx.fire_get("/api/eps/fuel-cells").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    for _ in 0..2 {
        let resp = ctx.fire_get("/api/eps/bus-voltage").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    let resp = ctx
        .fire_post("/api/sps/ignition", json!({ "burn_duration_s": 5.9 }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let resp = ctx.fire_delete("/api/sps/shutdown").await?;
    assert_eq!(resp.status().as_u16(), 204);

    for _ in 0..5 {
        let resp = ctx.fire_get("/api/rcs/thruster-status").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    for _ in 0..3 {
        let resp = ctx
            .fire_post(
                "/api/rcs/attitude-correction",
                json!({ "axis": "pitch", "degrees": 1.2 }),
            )
            .await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..4 {
        let resp = ctx
            .fire_post("/api/comms/downlink", json!({ "channel": "s-band" }))
            .await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..6 {
        let resp = ctx.fire_get("/api/thermal/radiator-status").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    let resp = ctx
        .fire_post("/api/docking/initiate", json!({ "target": "lunar-module" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let resp = ctx.fire_delete("/api/docking/separate").await?;
    assert_eq!(resp.status().as_u16(), 204);

    for _ in 0..7 {
        let resp = ctx.fire_get("/api/telemetry/downlink").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..2 {
        let resp = ctx.fire_get("/api/cabin/pressure").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    for _ in 0..3 {
        let resp = ctx
            .fire_put("/api/cabin/pressure", json!({ "pressure_psi": 5.2 }))
            .await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_playbooks", phase = "verification_begin", "all requests fired, verifying each playbook");

    guidance
        .verify(3, get_requested_for("/api/guidance/state"))
        .await;

    ecs.verify(2, get_requested_for("/api/ecs/cabin-atmosphere"))
        .await;
    ecs.verify(1, put_requested_for("/api/ecs/o2-flow")).await;

    eps.verify(4, get_requested_for("/api/eps/fuel-cells"))
        .await;
    eps.verify(2, get_requested_for("/api/eps/bus-voltage"))
        .await;

    sps.verify(1, post_requested_for("/api/sps/ignition")).await;
    sps.verify(1, delete_requested_for("/api/sps/shutdown"))
        .await;

    rcs.verify(5, get_requested_for("/api/rcs/thruster-status"))
        .await;
    rcs.verify(3, post_requested_for("/api/rcs/attitude-correction"))
        .await;

    comms
        .verify(4, post_requested_for("/api/comms/downlink"))
        .await;

    thermal
        .verify(6, get_requested_for("/api/thermal/radiator-status"))
        .await;

    docking
        .verify(1, post_requested_for("/api/docking/initiate"))
        .await;
    docking
        .verify(1, delete_requested_for("/api/docking/separate"))
        .await;

    telemetry
        .verify(7, get_requested_for("/api/telemetry/downlink"))
        .await;

    cabin
        .verify(2, get_requested_for("/api/cabin/pressure"))
        .await;
    cabin
        .verify(3, put_requested_for("/api/cabin/pressure"))
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_playbooks", phase = "scenario_ok", "all playbooks verified");
    Ok(())
}

async fn run_scenario_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx
        .http_dependency
        .playbook()
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .will_return(ok_json(json!({
            "stage": "terminal-count",
            "t_minus_s": 30,
            "go_for_launch": true
        })))
        .post("/api/vehicle/main-engine-start")
        .in_scenario("saturn-v-launch")
        .will_set_state_to("first-stage-flight")
        .will_return(ok_json(json!({
            "stage": "main-engine-start",
            "engines_running": 5,
            "thrust_kn": 33400
        })))
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .when_state_is("first-stage-flight")
        .will_return(ok_json(json!({
            "stage": "first-stage-flight",
            "altitude_km": 68,
            "velocity_mps": 2300,
            "max_q_passed": true
        })))
        .post("/api/vehicle/stage-separate")
        .in_scenario("saturn-v-launch")
        .when_state_is("first-stage-flight")
        .will_set_state_to("second-stage-flight")
        .will_return(ok_json(json!({
            "stage": "s-ic-separation",
            "interstage_jettisoned": true,
            "s_ii_ignition": true
        })))
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .when_state_is("second-stage-flight")
        .will_return(ok_json(json!({
            "stage": "second-stage-flight",
            "altitude_km": 185,
            "velocity_mps": 6900,
            "les_jettisoned": true
        })))
        .post("/api/vehicle/stage-separate")
        .in_scenario("saturn-v-launch")
        .when_state_is("second-stage-flight")
        .will_set_state_to("orbital-insertion")
        .will_return(ok_json(json!({
            "stage": "s-ii-separation",
            "s_ivb_ignition": true
        })))
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .when_state_is("orbital-insertion")
        .will_return(ok_json(json!({
            "stage": "parking-orbit",
            "altitude_km": 191,
            "velocity_mps": 7790,
            "orbit_achieved": true
        })))
        .run()
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "begin", "playbook applied, running launch sequence");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "terminal-count");
    assert_eq!(body["go_for_launch"], true);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "terminals_ok", "terminal count confirmed, go for launch");

    let resp = ctx
        .fire_post(
            "/api/vehicle/main-engine-start",
            json!({ "command": "ignition" }),
        )
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["engines_running"], 5);
    assert_eq!(body["thrust_kn"], 33400);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "engines_ok", "main engine start, engines running");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "first-stage-flight");
    assert_eq!(body["max_q_passed"], true);
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_http",
        case = "saturn_launch",
        phase = "max_q_passed",
        altitude_km = ?body["altitude_km"],
        "first-stage flight passed max-q",
    );

    let resp = ctx
        .fire_post("/api/vehicle/stage-separate", json!({ "stage": "S-IC" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["interstage_jettisoned"], true);
    assert_eq!(body["s_ii_ignition"], true);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "first_stage_sep", "first stage sep, upper stage ignition confirmed");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "second-stage-flight");
    assert_eq!(body["les_jettisoned"], true);
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_http",
        case = "saturn_launch",
        phase = "second_stage_ascent",
        altitude_km = ?body["altitude_km"],
        "second-stage ascent, tower equipment cleared",
    );

    let resp = ctx
        .fire_post("/api/vehicle/stage-separate", json!({ "stage": "S-II" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["s_ivb_ignition"], true);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "second_stage_sep", "second stage sep, restart burn begun");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "parking-orbit");
    assert_eq!(body["orbit_achieved"], true);
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_http",
        case = "saturn_launch",
        phase = "parking_orbit",
        altitude_km = ?body["altitude_km"],
        velocity_mps = ?body["velocity_mps"],
        "parking orbit achieved",
    );

    playbook
        .verify(4, get_requested_for("/api/vehicle/telemetry"))
        .await;
    playbook
        .verify(1, post_requested_for("/api/vehicle/main-engine-start"))
        .await;
    playbook
        .verify(2, post_requested_for("/api/vehicle/stage-separate"))
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch", phase = "scenario_ok", "all verifications passed");
    Ok(())
}

async fn run_sequence_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx
        .http_dependency
        .playbook()
        .get("/api/telemetry/altitude")
        .will_return(server_error())
        .then_return(status(503))
        .then_return(ok_json(json!({ "altitude_km": 185 })))
        .post("/api/telemetry/transmit")
        .will_return_in_sequence(vec![
            server_error(),
            server_error(),
            server_error(),
            created().with_json_body(json!({ "transmitted": true })),
        ])
        .run()
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", phase = "applied", "playbook applied");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 500);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "altitude_calls", attempt = 1, http_status = 500u16, "altitude probe response");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 503);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "altitude_calls", attempt = 2, http_status = 503u16, "altitude probe response");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["altitude_km"], 185);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "altitude_calls", attempt = 3, http_status = 200u16, phase_outcome = "success", "altitude probe response");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 200);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "altitude_calls", attempt = 4, http_status = 200u16, phase_outcome = "sticky_repeat", "altitude probe response");

    let resp = ctx
        .fire_post("/api/telemetry/transmit", json!({ "data": "ping" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 500);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "transmit_calls", attempt = 1, http_status = 500u16, "transmit probe response");

    let resp = ctx
        .fire_post("/api/telemetry/transmit", json!({ "data": "ping" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 500);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "transmit_calls", attempt = 2, http_status = 500u16, "transmit probe response");

    let resp = ctx
        .fire_post("/api/telemetry/transmit", json!({ "data": "ping" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 500);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "transmit_calls", attempt = 3, http_status = 500u16, "transmit probe response");

    let resp = ctx
        .fire_post("/api/telemetry/transmit", json!({ "data": "ping" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value =
        serde_json::from_str(&resp.text().await.map_err(|e| format!("read body: {e}"))?)
            .map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["transmitted"], true);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "transmit_calls", attempt = 4, http_status = 201u16, phase_outcome = "success", "transmit probe response");

    let resp = ctx
        .fire_post("/api/telemetry/transmit", json!({ "data": "ping" }))
        .await?;
    assert_eq!(resp.status().as_u16(), 201);
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", step = "transmit_calls", attempt = 5, http_status = 201u16, phase_outcome = "sticky_repeat", "transmit probe response");

    playbook
        .verify(4, get_requested_for("/api/telemetry/altitude"))
        .await;
    playbook
        .verify(5, post_requested_for("/api/telemetry/transmit"))
        .await;

    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "sticky_sequence", phase = "scenario_ok", "sequence verifications passed");
    Ok(())
}

#[tokio::test]
async fn http_dependency_spacecraft_inventory_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_spacecraft_inventory_test(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "spacecraft_inventory", phase = "case_ok", "case passed"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

#[tokio::test]
async fn http_dependency_scoped_playbooks_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_scoped_playbooks_test(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "spacecraft_systems", phase = "case_ok", "case passed"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

#[tokio::test]
async fn http_dependency_scenario_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_scenario_test(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "saturn_launch_wrapper", phase = "case_ok", "case passed"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

#[tokio::test]
async fn http_dependency_sequence_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_sequence_test(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "flaky_telemetry", phase = "case_ok", "case passed"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

async fn run_scoped_drop_test(ctx: &TestContext) -> Result<(), String> {
    {
        let scoped = ctx
            .http_dependency
            .playbook()
            .get("/api/mission/status")
            .will_return(ok_json(json!({ "status": "nominal" })))
            .run()
            .await;

        for _ in 0..2 {
            let resp = ctx.fire_get("/api/mission/status").await?;
            assert_eq!(resp.status().as_u16(), 200);
        }
        scoped
            .verify(2, get_requested_for("/api/mission/status"))
            .await;
        tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_drop", phase = "in_scope_ok", verified_requests = 2u32, "in-scope playbook verified");
    }

    let resp = ctx.fire_get("/api/mission/status").await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "expected 404 after ActivePlaybook was dropped and mapping deleted, got {}",
        resp.status().as_u16(),
    );
    tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_drop", phase = "after_drop", expected_status = 404u16, "mapping cleared after playbook drop");

    {
        let first = ctx
            .http_dependency
            .playbook()
            .get("/api/mission/heartbeat")
            .will_return(ok_json(json!({ "ok": true })))
            .run()
            .await;

        for _ in 0..3 {
            let resp = ctx.fire_get("/api/mission/heartbeat").await?;
            assert_eq!(resp.status().as_u16(), 200);
        }
        first
            .verify(3, get_requested_for("/api/mission/heartbeat"))
            .await;

        let second = ctx
            .http_dependency
            .playbook()
            .get("/api/mission/telemetry")
            .will_return(ok_json(json!({ "ok": true })))
            .run()
            .await;

        let resp = ctx.fire_get("/api/mission/telemetry").await?;
        assert_eq!(resp.status().as_u16(), 200);

        second
            .verify(1, get_requested_for("/api/mission/telemetry"))
            .await;
        first
            .verify(3, get_requested_for("/api/mission/heartbeat"))
            .await;
        tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "scoped_drop", phase = "concurrent_ok", "concurrent playbook scopes isolate mappings");
    }

    Ok(())
}

#[tokio::test]
async fn http_dependency_scoped_drop_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_scoped_drop_test(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => tracing::info!(suite = "crate_component", crate_under_test = "arena_http", case = "mission_control", phase = "case_ok", "case passed"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

async fn run_expect_called_exact_passes(ctx: &TestContext) -> Result<(), String> {
    let pb = ctx
        .http_dependency
        .playbook()
        .post("/api/expect/exact")
        .will_return(created())
        .expect_called(2)
        .run()
        .await;

    for _ in 0..2 {
        let resp = ctx.fire_post("/api/expect/exact", json!({})).await?;
        assert_eq!(resp.status().as_u16(), 201);
    }
    drop(pb);
    Ok(())
}

#[tokio::test]
async fn http_dependency_expect_called_exact_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_expect_called_exact_passes(&ctx))
        .catch_unwind()
        .await;
    ctx.stop().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(p) => std::panic::resume_unwind(p),
    }
}

async fn run_expect_at_least_passes(ctx: &TestContext) -> Result<(), String> {
    let pb = ctx
        .http_dependency
        .playbook()
        .get("/api/expect/atleast")
        .will_return(ok_json(json!({})))
        .expect_called_at_least(1)
        .run()
        .await;

    for _ in 0..3 {
        let resp = ctx.fire_get("/api/expect/atleast").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    drop(pb);
    Ok(())
}

#[tokio::test]
async fn http_dependency_expect_called_at_least_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_expect_at_least_passes(&ctx))
        .catch_unwind()
        .await;
    ctx.stop().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(p) => std::panic::resume_unwind(p),
    }
}

async fn run_expect_never_called_passes(ctx: &TestContext) -> Result<(), String> {
    let pb = ctx
        .http_dependency
        .playbook()
        .delete("/api/expect/never")
        .will_return(no_content())
        .expect_never_called()
        .run()
        .await;

    drop(pb);
    Ok(())
}

#[tokio::test]
async fn http_dependency_expect_never_called_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_expect_never_called_passes(&ctx))
        .catch_unwind()
        .await;
    ctx.stop().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(p) => std::panic::resume_unwind(p),
    }
}

async fn run_expect_called_fails_on_mismatch(ctx: &TestContext) -> Result<(), String> {
    let fire_url = "/api/expect/fail";
    let pb = ctx
        .http_dependency
        .playbook()
        .post(fire_url)
        .will_return(created())
        .expect_called(2)
        .run()
        .await;

    let resp = ctx.fire_post(fire_url, json!({})).await?;
    assert_eq!(resp.status().as_u16(), 201);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(pb)));
    match outcome {
        Ok(()) => Err(
            "expected drop to panic because expect_called(2) was only satisfied once".to_string(),
        ),
        Err(_) => Ok(()),
    }
}

#[tokio::test]
async fn http_dependency_expect_called_mismatch_panics_on_drop_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let outcome = std::panic::AssertUnwindSafe(run_expect_called_fails_on_mismatch(&ctx))
        .catch_unwind()
        .await;
    ctx.stop().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(p) => std::panic::resume_unwind(p),
    }
}

const EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = 21450..=21499;

fn ephemeral_host_tcp_port() -> u16 {
    arena_host::find_available_port::find_available_port(
        EPHEMERAL_PORT_RANGE,
        arena_host::find_available_port::PortSearchStrategy::Random,
    )
    .unwrap_or_else(|| {
        panic!(
            "no available port found in range {}..={}",
            EPHEMERAL_PORT_RANGE.start(), EPHEMERAL_PORT_RANGE.end()
        )
    })
}

fn stub_https_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("reqwest client for https stub")
}

#[tokio::test]
async fn http_dependency_https_listener_stub_roundtrip_component_test() {
    init_test_logging();

    let http_host_port = ephemeral_host_tcp_port();
    let https_host_port = ephemeral_host_tcp_port();

    let mut dep = HttpDependency::builder("https listener stub")
        .with_port(http_host_port)
        .https()
        .listener_container_port(8443)
        .host_port(https_host_port)
        .done()
        .build();

    dep.start().await;

    let https_origin = dep
        .https_base_url()
        .expect("https_base_url after start")
        .to_string();
    assert!(
        https_origin.starts_with("https://"),
        "expected https origin, got {https_origin}"
    );

    let http_origin = dep.base_url().expect("http base_url");
    assert!(
        http_origin.starts_with("http://"),
        "expected http origin alongside https, got {http_origin}"
    );

    let admin_url = dep.admin_url().expect("admin_url");
    assert!(
        admin_url.starts_with("http://"),
        "expected admin on http when http listener is enabled, got {admin_url}"
    );

    let pb = dep
        .playbook()
        .get("/api/https-stub-check")
        .will_return(ok_json(json!({ "via_tls": true })))
        .run()
        .await;

    let client = stub_https_client();
    let resp = client
        .get(format!("{https_origin}/api/https-stub-check"))
        .send()
        .await
        .expect("GET over https");
    assert!(
        resp.status().is_success(),
        "GET https status {}",
        resp.status()
    );
    let text = resp.text().await.expect("response body");
    let body: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("response json: {e}; body: {text}"));
    assert_eq!(body["via_tls"], true);

    pb.verify(1, get_requested_for("/api/https-stub-check"))
        .await;
    drop(pb);
    dep.stop().await;
}
