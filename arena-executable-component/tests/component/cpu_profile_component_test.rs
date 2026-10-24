#[path = "../support/profile_subjects.rs"]
mod profile_subjects;
#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::component::RunnableComponent;
use arena::lifecycle::RunnableState;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::ExecutableComponent;
use profile_subjects::{await_completed_pass, marker_path, subject_path, PASS_MILLIS};
use recorded_profiles::start_recording;

#[tokio::test]
async fn stop_pyspy_wrapped_profile_attached_reports_folded_stacks() {
    let recording = start_recording();
    let marker = marker_path("pyspy");

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("script", subject_path("tools/profile_subjects/busy_loop.py"))
        .with_runtime_arg("work", PASS_MILLIS.to_string())
        .with_runtime_arg("marker", marker.to_string_lossy().into_owned())
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler(None)
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    if let Err(reason) = await_completed_pass(&marker, &component).await {
        component.force_stop().await;
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }
    component.stop().await.expect("stop component");
    let _ = std::fs::remove_file(&marker);

    let folded = recording
        .recorded
        .folded_for(&component_id)
        .expect("folded stacks were not reported");
    assert!(
        folded.contains("bar"),
        "the subject's hotspot was not sampled, got: {folded}"
    );
}

#[tokio::test]
async fn stop_pyspy_wrapped_profile_attached_reports_the_full_transition_sequence() {
    let recording = start_recording();
    let marker = marker_path("pyspy-transitions");

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-transitions-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("script", subject_path("tools/profile_subjects/busy_loop.py"))
        .with_runtime_arg("work", PASS_MILLIS.to_string())
        .with_runtime_arg("marker", marker.to_string_lossy().into_owned())
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler(None)
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    if let Err(reason) = await_completed_pass(&marker, &component).await {
        component.force_stop().await;
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }
    component.stop().await.expect("stop component");
    let _ = std::fs::remove_file(&marker);

    assert_eq!(
        recording.recorded.states_for(&component_id),
        vec![
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped,
        ]
    );
}

#[tokio::test]
async fn stop_async_profiler_arg_augmented_profile_attached_reports_folded_stacks() {
    let recording = start_recording();
    let marker = marker_path("async-profiler");

    let mut component = ExecutableComponent::builder("cpu-profile-asprof-test")
        .with_build_tool(BuildTool::Maven)
        .with_executable_path("/usr/bin/java")
        .with_runtime_arg("fixture", subject_path("tools/profile_subjects/BusyLoop.java"))
        .with_runtime_arg("work", PASS_MILLIS.to_string())
        .with_runtime_arg("marker", marker.to_string_lossy().into_owned())
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler(None)
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    if let Err(reason) = await_completed_pass(&marker, &component).await {
        component.force_stop().await;
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }
    component.stop().await.expect("stop component");
    let _ = std::fs::remove_file(&marker);

    let folded = recording
        .recorded
        .folded_for(&component_id)
        .expect("folded stacks were not reported");
    assert!(
        folded.contains("bar"),
        "the subject's hotspot was not sampled, got: {folded}"
    );
}
