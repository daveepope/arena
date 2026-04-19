use arena::dependency::RunnableDependency;
use arena_http::{
    ActivePlaybook, HttpDependency,
    ok_json, created, no_content, server_error, status,
    get_requested_for, post_requested_for, put_requested_for, delete_requested_for,
};
use futures::FutureExt;
use serde_json::json;

fn init_test_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
}

struct TestContext {
    http_dependency: HttpDependency,
    base_url: String,
    client: reqwest::Client,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        log::info!("[component-test] starting HttpDependency");
        let mut http_dependency = HttpDependency::builder("").build();
        http_dependency.start().await;

        let base_url = http_dependency
            .base_url()
            .ok_or_else(|| "http base_url missing after start()".to_string())?
            .to_string();

        log::info!("[component-test] http dependency started (base_url={base_url})");
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

    async fn fire_post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response, String> {
        self.client
            .post(self.url(path))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("POST {path} failed: {e}"))
    }

    async fn fire_put(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response, String> {
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
    ctx.http_dependency.playbook()
        .get("/api/guidance/state")
            .will_return(ok_json(json!({
                "mode": "free-return",
                "attitude_deg": { "roll": 0.2, "pitch": -1.4, "yaw": 0.0 }
            })))
        .run().await
}

async fn setup_ecs_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/ecs/cabin-atmosphere")
            .will_return(ok_json(json!({
                "o2_psi": 4.8,
                "co2_mmhg": 3.1,
                "temp_f": 72
            })))
        .put("/api/ecs/o2-flow")
            .will_return(ok_json(json!({ "o2_flow_lb_hr": 0.96 })))
        .run().await
}

async fn setup_eps_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/eps/fuel-cells")
            .will_return(ok_json(json!({
                "cells": [
                    { "id": 1, "voltage": 29.0, "amps": 20.5 },
                    { "id": 2, "voltage": 29.1, "amps": 19.8 },
                    { "id": 3, "voltage": 28.9, "amps": 20.1 }
                ]
            })))
        .get("/api/eps/bus-voltage")
            .will_return(ok_json(json!({ "main_bus_a_v": 29.0, "main_bus_b_v": 29.1 })))
        .run().await
}

async fn setup_sps_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .post("/api/sps/ignition")
            .will_return(ok_json(json!({
                "burn_duration_s": 5.9,
                "delta_v_fps": 32.4
            })))
        .delete("/api/sps/shutdown")
            .will_return(no_content())
        .run().await
}

async fn setup_rcs_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/rcs/thruster-status")
            .will_return(ok_json(json!({
                "quads": {
                    "a": "nominal", "b": "nominal",
                    "c": "nominal", "d": "nominal"
                }
            })))
        .post("/api/rcs/attitude-correction")
            .will_return(ok_json(json!({ "correction_applied": true })))
        .run().await
}

async fn setup_comms_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .post("/api/comms/downlink")
            .will_return(ok_json(json!({
                "signal": "s-band",
                "station": "goldstone",
                "strength_dbm": -120
            })))
        .run().await
}

async fn setup_thermal_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/thermal/radiator-status")
            .will_return(ok_json(json!({
                "radiator_1_f": 45,
                "radiator_2_f": 42,
                "coolant_flow_ok": true
            })))
        .run().await
}

async fn setup_docking_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .post("/api/docking/initiate")
            .will_return(ok_json(json!({
                "target": "lunar-module",
                "capture_latches": "engaged"
            })))
        .delete("/api/docking/separate")
            .will_return(no_content())
        .run().await
}

async fn setup_telemetry_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/telemetry/downlink")
            .will_return(ok_json(json!({
                "bit_rate_kbps": 51.2,
                "frames_sent": 14832
            })))
        .run().await
}

async fn setup_cabin_playbook(ctx: &TestContext) -> ActivePlaybook {
    ctx.http_dependency.playbook()
        .get("/api/cabin/pressure")
            .will_return(ok_json(json!({ "pressure_psi": 5.0, "nominal": true })))
        .put("/api/cabin/pressure")
            .will_return(ok_json(json!({ "pressure_psi": 5.2, "nominal": true })))
        .run().await
}

async fn run_spacecraft_inventory_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx.http_dependency.playbook()
        .get("/api/spacecraft")
            .will_return(
                ok_json(json!({
                    "spacecraft": [
                        { "id": 1, "name": "Eagle Transporter", "class": "freighter" },
                        { "id": 2, "name": "Discovery One", "class": "exploration" }
                    ]
                }))
            )
        .get("/api/spacecraft/1")
            .will_return(
                ok_json(json!({
                    "id": 1,
                    "name": "Eagle Transporter",
                    "class": "freighter",
                    "crew_capacity": 4
                }))
            )
        .post("/api/spacecraft")
            .will_return(
                created()
                    .with_json_body(json!({ "id": 3, "name": "Rocinante", "class": "corvette" }))
                    .with_header("Location", "/api/spacecraft/3")
            )
        .put("/api/spacecraft/1")
            .will_return(
                ok_json(json!({
                    "id": 1,
                    "name": "Eagle Transporter",
                    "class": "freighter",
                    "crew_capacity": 6
                }))
            )
        .delete("/api/spacecraft/2")
            .will_return(no_content())
        .run()
        .await;

    log::info!("[spacecraft-inventory] playbook applied, calling endpoints");

    let resp = ctx.fire_get("/api/spacecraft").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["spacecraft"].as_array().unwrap().len(), 2);

    let resp = ctx.fire_get("/api/spacecraft/1").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["name"], "Eagle Transporter");

    let resp = ctx.fire_post("/api/spacecraft", json!({ "name": "Rocinante", "class": "corvette" })).await?;
    assert_eq!(resp.status().as_u16(), 201);
    let location = resp.headers().get("Location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(location, "/api/spacecraft/3");

    let resp = ctx.fire_put("/api/spacecraft/1", json!({ "crew_capacity": 6 })).await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["crew_capacity"], 6);

    let resp = ctx.fire_delete("/api/spacecraft/2").await?;
    assert_eq!(resp.status().as_u16(), 204);

    playbook.verify(1, get_requested_for("/api/spacecraft")).await;
    playbook.verify(1, get_requested_for("/api/spacecraft/1")).await;
    playbook.verify(1, post_requested_for("/api/spacecraft")).await;
    playbook.verify(1, put_requested_for("/api/spacecraft/1")).await;
    playbook.verify(1, delete_requested_for("/api/spacecraft/2")).await;

    let post_requests = playbook.find_requests(post_requested_for("/api/spacecraft")).await;
    assert_eq!(post_requests.len(), 1);
    assert!(post_requests[0].body.contains("Rocinante"));

    log::info!("[spacecraft-inventory] all verifications passed");
    Ok(())
}

async fn run_scoped_playbooks_test(ctx: &TestContext) -> Result<(), String> {
    let guidance  = setup_guidance_playbook(ctx).await;
    let ecs       = setup_ecs_playbook(ctx).await;
    let eps       = setup_eps_playbook(ctx).await;
    let sps       = setup_sps_playbook(ctx).await;
    let rcs       = setup_rcs_playbook(ctx).await;
    let comms     = setup_comms_playbook(ctx).await;
    let thermal   = setup_thermal_playbook(ctx).await;
    let docking   = setup_docking_playbook(ctx).await;
    let telemetry = setup_telemetry_playbook(ctx).await;
    let cabin     = setup_cabin_playbook(ctx).await;

    log::info!("[scoped-playbooks] 10 playbooks applied, firing requests");

    for _ in 0..3 {
        let resp = ctx.fire_get("/api/guidance/state").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..2 {
        let resp = ctx.fire_get("/api/ecs/cabin-atmosphere").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    let resp = ctx.fire_put("/api/ecs/o2-flow", json!({ "o2_flow_lb_hr": 0.96 })).await?;
    assert_eq!(resp.status().as_u16(), 200);

    for _ in 0..4 {
        let resp = ctx.fire_get("/api/eps/fuel-cells").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    for _ in 0..2 {
        let resp = ctx.fire_get("/api/eps/bus-voltage").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    let resp = ctx.fire_post("/api/sps/ignition", json!({ "burn_duration_s": 5.9 })).await?;
    assert_eq!(resp.status().as_u16(), 200);
    let resp = ctx.fire_delete("/api/sps/shutdown").await?;
    assert_eq!(resp.status().as_u16(), 204);

    for _ in 0..5 {
        let resp = ctx.fire_get("/api/rcs/thruster-status").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }
    for _ in 0..3 {
        let resp = ctx.fire_post("/api/rcs/attitude-correction", json!({ "axis": "pitch", "degrees": 1.2 })).await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..4 {
        let resp = ctx.fire_post("/api/comms/downlink", json!({ "channel": "s-band" })).await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    for _ in 0..6 {
        let resp = ctx.fire_get("/api/thermal/radiator-status").await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    let resp = ctx.fire_post("/api/docking/initiate", json!({ "target": "lunar-module" })).await?;
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
        let resp = ctx.fire_put("/api/cabin/pressure", json!({ "pressure_psi": 5.2 })).await?;
        assert_eq!(resp.status().as_u16(), 200);
    }

    log::info!("[scoped-playbooks] all requests fired, verifying each playbook");

    guidance.verify(3, get_requested_for("/api/guidance/state")).await;

    ecs.verify(2, get_requested_for("/api/ecs/cabin-atmosphere")).await;
    ecs.verify(1, put_requested_for("/api/ecs/o2-flow")).await;

    eps.verify(4, get_requested_for("/api/eps/fuel-cells")).await;
    eps.verify(2, get_requested_for("/api/eps/bus-voltage")).await;

    sps.verify(1, post_requested_for("/api/sps/ignition")).await;
    sps.verify(1, delete_requested_for("/api/sps/shutdown")).await;

    rcs.verify(5, get_requested_for("/api/rcs/thruster-status")).await;
    rcs.verify(3, post_requested_for("/api/rcs/attitude-correction")).await;

    comms.verify(4, post_requested_for("/api/comms/downlink")).await;

    thermal.verify(6, get_requested_for("/api/thermal/radiator-status")).await;

    docking.verify(1, post_requested_for("/api/docking/initiate")).await;
    docking.verify(1, delete_requested_for("/api/docking/separate")).await;

    telemetry.verify(7, get_requested_for("/api/telemetry/downlink")).await;

    cabin.verify(2, get_requested_for("/api/cabin/pressure")).await;
    cabin.verify(3, put_requested_for("/api/cabin/pressure")).await;

    log::info!("[scoped-playbooks] all 10 playbooks verified");
    Ok(())
}

async fn run_scenario_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx.http_dependency.playbook()
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

    log::info!("[scenario] playbook applied, running Saturn V launch sequence");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "terminal-count");
    assert_eq!(body["go_for_launch"], true);
    log::info!("[scenario] terminal count confirmed, go for launch");

    let resp = ctx.fire_post("/api/vehicle/main-engine-start", json!({ "command": "ignition" })).await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["engines_running"], 5);
    assert_eq!(body["thrust_kn"], 33400);
    log::info!("[scenario] main engine start, all 5 F-1 engines running");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "first-stage-flight");
    assert_eq!(body["max_q_passed"], true);
    log::info!("[scenario] first-stage flight, max-Q passed at {} km", body["altitude_km"]);

    let resp = ctx.fire_post("/api/vehicle/stage-separate", json!({ "stage": "S-IC" })).await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["interstage_jettisoned"], true);
    assert_eq!(body["s_ii_ignition"], true);
    log::info!("[scenario] S-IC separated, S-II ignition confirmed");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "second-stage-flight");
    assert_eq!(body["les_jettisoned"], true);
    log::info!("[scenario] second-stage flight, LES jettisoned at {} km", body["altitude_km"]);

    let resp = ctx.fire_post("/api/vehicle/stage-separate", json!({ "stage": "S-II" })).await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["s_ivb_ignition"], true);
    log::info!("[scenario] S-II separated, S-IVB first burn started");

    let resp = ctx.fire_get("/api/vehicle/telemetry").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["stage"], "parking-orbit");
    assert_eq!(body["orbit_achieved"], true);
    log::info!("[scenario] parking orbit achieved at {} km, {} m/s", body["altitude_km"], body["velocity_mps"]);

    playbook.verify(4, get_requested_for("/api/vehicle/telemetry")).await;
    playbook.verify(1, post_requested_for("/api/vehicle/main-engine-start")).await;
    playbook.verify(2, post_requested_for("/api/vehicle/stage-separate")).await;

    log::info!("[scenario] all verifications passed — orbit achieved");
    Ok(())
}

async fn run_sequence_test(ctx: &TestContext) -> Result<(), String> {
    let playbook = ctx.http_dependency.playbook()
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

    log::info!("[sequence] playbook applied");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 500);
    log::info!("[sequence] altitude call 1: 500");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 503);
    log::info!("[sequence] altitude call 2: 503");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["altitude_km"], 185);
    log::info!("[sequence] altitude call 3: 200 (success)");

    let resp = ctx.fire_get("/api/telemetry/altitude").await?;
    assert_eq!(resp.status().as_u16(), 200);
    log::info!("[sequence] altitude call 4: 200 (still sticks)");

    let resp = ctx.fire_post("/api/telemetry/transmit", json!({ "data": "ping" })).await?;
    assert_eq!(resp.status().as_u16(), 500);
    log::info!("[sequence] transmit call 1: 500");

    let resp = ctx.fire_post("/api/telemetry/transmit", json!({ "data": "ping" })).await?;
    assert_eq!(resp.status().as_u16(), 500);
    log::info!("[sequence] transmit call 2: 500");

    let resp = ctx.fire_post("/api/telemetry/transmit", json!({ "data": "ping" })).await?;
    assert_eq!(resp.status().as_u16(), 500);
    log::info!("[sequence] transmit call 3: 500");

    let resp = ctx.fire_post("/api/telemetry/transmit", json!({ "data": "ping" })).await?;
    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = serde_json::from_str(
        &resp.text().await.map_err(|e| format!("read body: {e}"))?
    ).map_err(|e| format!("parse json: {e}"))?;
    assert_eq!(body["transmitted"], true);
    log::info!("[sequence] transmit call 4: 201 (success)");

    let resp = ctx.fire_post("/api/telemetry/transmit", json!({ "data": "ping" })).await?;
    assert_eq!(resp.status().as_u16(), 201);
    log::info!("[sequence] transmit call 5: 201 (still sticks)");

    playbook.verify(4, get_requested_for("/api/telemetry/altitude")).await;
    playbook.verify(5, post_requested_for("/api/telemetry/transmit")).await;

    log::info!("[sequence] all verifications passed");
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
        Ok(Ok(())) => log::info!("[component-test] spacecraft-inventory ok"),
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
        Ok(Ok(())) => log::info!("[component-test] spacecraft-systems ok"),
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
        Ok(Ok(())) => log::info!("[component-test] launch-sequence ok"),
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
        Ok(Ok(())) => log::info!("[component-test] flaky-telemetry ok"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

async fn run_scoped_drop_test(ctx: &TestContext) -> Result<(), String> {
    {
        let scoped = ctx.http_dependency.playbook()
            .get("/api/mission/status")
                .will_return(ok_json(json!({ "status": "nominal" })))
            .run()
            .await;

        for _ in 0..2 {
            let resp = ctx.fire_get("/api/mission/status").await?;
            assert_eq!(resp.status().as_u16(), 200);
        }
        scoped.verify(2, get_requested_for("/api/mission/status")).await;
        log::info!("[scoped-drop] in-scope: 2 requests recorded and verified");
    }

    let resp = ctx.fire_get("/api/mission/status").await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "expected 404 after ActivePlaybook was dropped and mapping deleted, got {}",
        resp.status().as_u16(),
    );
    log::info!("[scoped-drop] after drop: mapping is gone (404)");

    {
        let first = ctx.http_dependency.playbook()
            .get("/api/mission/heartbeat")
                .will_return(ok_json(json!({ "ok": true })))
            .run()
            .await;

        for _ in 0..3 {
            let resp = ctx.fire_get("/api/mission/heartbeat").await?;
            assert_eq!(resp.status().as_u16(), 200);
        }
        first.verify(3, get_requested_for("/api/mission/heartbeat")).await;

        let second = ctx.http_dependency.playbook()
            .get("/api/mission/telemetry")
                .will_return(ok_json(json!({ "ok": true })))
            .run()
            .await;

        let resp = ctx.fire_get("/api/mission/telemetry").await?;
        assert_eq!(resp.status().as_u16(), 200);

        second.verify(1, get_requested_for("/api/mission/telemetry")).await;
        first.verify(3, get_requested_for("/api/mission/heartbeat")).await;
        log::info!("[scoped-drop] concurrent playbooks verify independently (scope-local)");
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
        Ok(Ok(())) => log::info!("[component-test] mission-control ok"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

async fn run_expect_called_exact_passes(ctx: &TestContext) -> Result<(), String> {
    let pb = ctx.http_dependency.playbook()
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
    let pb = ctx.http_dependency.playbook()
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
    let pb = ctx.http_dependency.playbook()
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
    let pb = ctx.http_dependency.playbook()
        .post(fire_url)
            .will_return(created())
            .expect_called(2)
        .run()
        .await;

    let resp = ctx.fire_post(fire_url, json!({})).await?;
    assert_eq!(resp.status().as_u16(), 201);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(pb)));
    match outcome {
        Ok(()) => Err("expected drop to panic because expect_called(2) was only satisfied once".to_string()),
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
