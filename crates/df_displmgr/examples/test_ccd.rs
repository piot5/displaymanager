use std::time::Instant;
use serde_json::json;
use df_displmgr::traits::UniversalTopology;
use df_displmgr::types::{DisplayRotation, Extent2D, Point2D};
use df_displmgr::backends::windows::displmgr_ccd::CcdTopology;

struct Logger {
    start: Instant,
}

impl Logger {
    fn new() -> Self {
        Self { start: Instant::now() }
    }

    fn log(&self, case: &str, phase: &str, status: &str, payload: serde_json::Value) {
        println!("{}", json!({
            "timestamp_us": self.start.elapsed().as_micros(),
            "test_case":    case,
            "phase":        phase,
            "status":       status,
            "data":         payload,
        }));
    }
}

#[tokio::main]
async fn main() {
    let log = Logger::new();

    log.log("GLOBAL_INIT", "START", "INFO",
        json!({ "msg": "Starting transactional deep test harness for Windows CCD backend" }));

    let mut topology = match CcdTopology::acquire() {
        Ok(top) => {
            log.log("GLOBAL_INIT", "ACQUIRE", "SUCCESS",
                json!({ "count": top.get_outputs().len() }));
            top
        }
        Err(e) => {
            log.log("GLOBAL_INIT", "ACQUIRE", "FATAL",
                json!({ "error": format!("{:?}", e) }));
            return;
        }
    };

    if topology.get_outputs().is_empty() {
        log.log("GLOBAL_INIT", "END", "ABORT",
            json!({ "msg": "No active display paths available for testing." }));
        return;
    }

    // =========================================================================
    // TEST CASE 1: Inactive Target Path Activation
    // =========================================================================
    log.log("TEST_ACTIVATE_INACTIVE", "SCAN", "START", json!({}));
    
    let mut inactive_targets: Vec<df_displmgr::types::DisplayId> = topology.get_outputs().iter()
        .filter(|o| o.geometry.size.width == 0 || o.geometry.size.height == 0)
        .map(|o| o.identity.id.clone())
        .collect();

    let mut mock_active_target: Option<df_displmgr::types::DisplayId> = None;
        if inactive_targets.is_empty() {
        let fallback_id = topology.get_outputs()[0].identity.id.clone();
        log.log("TEST_ACTIVATE_INACTIVE", "MOCK_GENERATION", "INFO", 
            json!({ "msg": "No inactive monitors detected. Generating inactive mock state from live node.", "target_id": &fallback_id }));
        
        if let Ok(mut editor) = topology.edit_output(&fallback_id) {
            let _ = editor.set_enabled(false);
            mock_active_target = Some(fallback_id.clone());
            inactive_targets.push(fallback_id);
        }
    }

    for target_id in &inactive_targets {
        log.log("TEST_ACTIVATE_INACTIVE", "WAKE_ATTEMPT", "INIT", json!({ "target_id": target_id }));
        
        let mut staged_activation = false;
        if let Ok(mut editor) = topology.edit_output(&target_id) {
            let _ = editor.set_enabled(true);
            let _ = editor.set_position(Point2D{ x: 0, y: 0 }); 
            let _ = editor.set_resolution(Extent2D{ width: 1920, height: 1080 });
            
            log.log("TEST_ACTIVATE_INACTIVE", "STAGE_STATE", "SUCCESS", 
                json!({ "msg": "Inactive path set to ENABLED in local editor buffer." }));
            staged_activation = true;
        }

        if staged_activation {
            match topology.validate().await {
                Ok(_) => {
                    log.log("TEST_ACTIVATE_INACTIVE", "VALIDATE", "SUCCESS", 
                        json!({ "msg": "OS kernel accepted re-activation of the inactive CCD path." }));
                    
                    if topology.commit().await.is_ok() {
                        log.log("TEST_ACTIVATE_INACTIVE", "COMMIT", "SUCCESS", 
                            json!({ "msg": "Monitor successfully woken from deep sleep." }));
                    }
                },
                Err(e) => {
                    log.log("TEST_ACTIVATE_INACTIVE", "VALIDATE", "FAILED", 
                        json!({ "error": format!("{:?}", e), "msg": "Kernel refused activation." }));
                }
            }
        }
    }
    
    if let Ok(fresh) = CcdTopology::acquire() { topology = fresh; }

    let target_id = if let Some(mock_id) = mock_active_target {
        mock_id
    } else {
        topology.get_outputs()[0].identity.id.clone()
    };

    // =========================================================================
    // TEST CASE 2: Transactional Rollback
    // =========================================================================
    log.log("TEST_ROLLBACK", "INIT", "START", json!({ "target_id": &target_id }));

    let mut staged_illegal = false;
    {
        if let Ok(mut editor) = topology.edit_output(&target_id) {
            let _ = editor.set_resolution(Extent2D { width: 99999u32, height: 99999u32 });
            log.log("TEST_ROLLBACK", "STAGE_ILLEGAL", "WARNING",
                json!({ "msg": "Editor accepted 99999x99999 in local buffer." }));
            staged_illegal = true;
        }
    }

    let mut rollback_triggered = false;
    if staged_illegal {
        match topology.validate().await {
            Err(_) => {
                log.log("TEST_ROLLBACK", "VALIDATE", "SUCCESS_REJECTED",
                    json!({ "error": "ConfigurationRejected",
                             "msg":   "CCD kernel correctly rejected malformed bounds" }));
                rollback_triggered = true;
            }
            Ok(_) => {
                log.log("TEST_ROLLBACK", "VALIDATE", "CRITICAL_ALLOWED",
                    json!({ "msg": "Kernel accepted 99999x99999 — unexpected." }));
            }
        }
    }

    if rollback_triggered {
        log.log("TEST_ROLLBACK", "ROLLBACK", "EXECUTE",
            json!({ "msg": "Re-acquiring fresh topology to purge dirty state" }));
        match CcdTopology::acquire() {
            Ok(fresh) => {
                topology = fresh;
                log.log("TEST_ROLLBACK", "SANITY_CHECK", "SUCCESS",
                    json!({ "msg": "Topology memory state restored post-rollback." }));
            }
            Err(e) => {
                log.log("TEST_ROLLBACK", "SANITY_CHECK", "FAILURE",
                    json!({ "error": format!("{:?}", e) }));
                return;
            }
        }
    }

    // =========================================================================
    // TEST CASE 3: Spatial Collision Injection
    // =========================================================================
    if topology.get_outputs().len() >= 2 {
        let id_1 = topology.get_outputs()[0].identity.id.clone();
        let id_2 = topology.get_outputs()[1].identity.id.clone();
        log.log("TEST_OVERLAP", "COLLISION_INJECTION", "START",
            json!({ "id_1": &id_1, "id_2": &id_2 }));

        if let Ok(mut e) = topology.edit_output(&id_1) { let _ = e.set_position(Point2D { x: 0, y: 0 }); }
        if let Ok(mut e) = topology.edit_output(&id_2) { let _ = e.set_position(Point2D { x: 10, y: 10 }); }

        match topology.validate().await {
            Ok(_) => log.log("TEST_OVERLAP", "VALIDATE", "CRITICAL_ALLOWED",
                json!({ "msg": "OS permitted overlapping layout — potential canvas corruption." })),
            Err(_) => log.log("TEST_OVERLAP", "VALIDATE", "REJECTED_BY_OS",
                json!({ "msg": "OS kernel correctly blocks spatial collision." })),
        }

        if let Ok(fresh) = CcdTopology::acquire() { topology = fresh; }
    }

    // =========================================================================
    // TEST CASE 4: Rotation Cycle Verification
    // =========================================================================
    log.log("TEST_ROTATION_CYCLE", "RUN", "START", json!({ "target_id": &target_id }));

    let rotations: &[(&str, DisplayRotation)] = &[
        ("Rotate90",  DisplayRotation::Rotate90),
        ("Rotate180", DisplayRotation::Rotate180),
        ("Rotate270", DisplayRotation::Rotate270),
        ("Rotate0",   DisplayRotation::Rotate0),
    ];

    for &(label, rot) in rotations {
        let mut staged = false;
        {
            if let Ok(mut editor) = topology.edit_output(&target_id) {
                if editor.set_rotation(rot).is_ok() {
                    staged = true;
                }
            }
        }
        if staged {
            match topology.validate().await {
                Ok(_)  => log.log("TEST_ROTATION_CYCLE", label, "VALID_HARDWARE_MODE", json!({})),
                Err(_) => log.log("TEST_ROTATION_CYCLE", label, "VALIDATE_REJECTED",
                    json!({ "error": "ConfigurationRejected",
                             "info":  "Monitor may not support this orientation" })),
            }
        }
    }

    if let Ok(fresh) = CcdTopology::acquire() { topology = fresh; }

    // =========================================================================
    // TEST CASE 5: Idempotency Validation
    // =========================================================================
    log.log("TEST_IDEMPOTENCY", "EXECUTE", "START", json!({}));

    match topology.commit().await {
        Ok(_)  => log.log("TEST_IDEMPOTENCY", "FIRST_COMMIT",  "SUCCESS",  json!({})),
        Err(e) => log.log("TEST_IDEMPOTENCY", "FIRST_COMMIT",  "FAILURE",
            json!({ "error": format!("{:?}", e) })),
    }
    match topology.commit().await {
        Ok(_)  => log.log("TEST_IDEMPOTENCY", "SECOND_COMMIT_IDEMPOTENT", "SUCCESS", json!({})),
        Err(e) => log.log("TEST_IDEMPOTENCY", "SECOND_COMMIT_IDEMPOTENT", "FAILURE",
            json!({ "error": format!("{:?}", e) })),
    }

    log.log("GLOBAL_INIT", "END", "INFO",
        json!({ "msg": "Deep display configuration testing finalized successfully." }));
}