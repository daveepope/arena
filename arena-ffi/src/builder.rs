use arena::{Component, Dependency, Encounter, EncounterTrait};
use arena_container_component::container_component::ContainerComponent;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_executable_component::BuildTool;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;

use crate::parse::{
    BuildToolJson, ComponentJson, ContainerJson, DependencyJson, EncounterJson, ExecJson,
};
use crate::readiness_json::{
    apply_readiness_checks_to_container, apply_readiness_checks_to_exec,
    collect_container_readiness, collect_exec_readiness,
};

const DEFAULT_NETWORK: &str = "arena-network";
const DEFAULT_ENCOUNTER_NAME: &str = "arena-encounter";

pub(super) async fn build_encounter_async(json: &EncounterJson) -> Box<dyn EncounterTrait> {
    let network = json.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    let encounter_name = json.encounter_name.as_deref().unwrap_or(DEFAULT_ENCOUNTER_NAME);
    let dependencies = build_dependencies(json, network);
    let components = build_components_async(json).await;
    Box::new(Encounter::new(encounter_name, dependencies, components))
}

fn build_dependencies(json: &EncounterJson, network: &str) -> Vec<Dependency> {
    let deps = json.dependencies.as_deref().unwrap_or(&[]);
    deps.iter()
        .filter_map(|d| build_dependency(d, network))
        .collect()
}

fn build_dependency(json: &DependencyJson, network: &str) -> Option<Dependency> {
    match json {
        DependencyJson::Postgres(p) => {
            let mut builder = PostgresDependency::builder(&p.identifier)
                .with_image(p.image.as_deref().unwrap_or("14.20-trixie"));
            if let Some(ref image_name) = p.image_name {
                builder = builder.with_image_name(image_name);
            }
            let dep = builder
                .with_port(p.port.unwrap_or(5432))
                .with_database_name(p.database_name.as_deref().unwrap_or("arena_db"))
                .with_database_username(p.database_username.as_deref().unwrap_or("arena_user"))
                .with_database_password(p.database_password.as_deref().unwrap_or("postgres"))
                .with_container_name(
                    p.container_name
                        .as_deref()
                        .unwrap_or(&format!("arena-postgres-{}", p.identifier.replace(' ', "-"))),
                )
                .with_network(network)
                .with_startup_sql_scripts(p.startup_sql_scripts.clone().unwrap_or_default())
                .build();
            Some(Box::new(dep))
        }
        DependencyJson::Kafka(k) => {
            let flavor = match k.flavor.as_deref() {
                Some("confluent") => KafkaFlavor::Confluent,
                _ => KafkaFlavor::ApacheNative,
            };
            let mut builder = KafkaDependency::builder(&k.identifier)
                .with_flavor(flavor)
                .with_port(k.port.unwrap_or(9092))
                .with_container_name(
                    k.container_name
                        .as_deref()
                        .unwrap_or(&format!("arena-kafka-{}", k.identifier.replace(' ', "-"))),
                )
                .with_network(network);
            if let Some(ref image_name) = k.image_name {
                builder = builder.with_image_name(image_name);
            }
            for topic in k.topics.as_deref().unwrap_or(&[]) {
                builder = builder.with_topic(topic);
            }
            let dep = builder.build();
            Some(Box::new(dep))
        }
    }
}

async fn build_components_async(json: &EncounterJson) -> Vec<Component> {
    let comps = json.components.as_deref().unwrap_or(&[]);
    let mut out = Vec::with_capacity(comps.len());
    for c in comps {
        let comp: Component = match c {
            ComponentJson::Exec(e) => build_exec_component(e),
            ComponentJson::Container(ct) => build_container_component(ct).await,
        };
        out.push(comp);
    }
    out
}

fn build_tool_from_json(json: &BuildToolJson) -> BuildTool {
    match json {
        BuildToolJson::Simple(s) => match s.as_str() {
            "cargo" => BuildTool::Cargo,
            "maven" => BuildTool::Maven,
            "gradle" => BuildTool::Gradle,
            "dotnet" => BuildTool::Dotnet,
            "make" => BuildTool::Make,
            "cmake" => BuildTool::CMake,
            _ => BuildTool::Cargo,
        },
        BuildToolJson::Custom { command, args } => BuildTool::Custom {
            command: command.clone(),
            args: args.clone(),
        },
    }
}

async fn build_container_component(json: &ContainerJson) -> Component {
    let mut builder = ContainerComponent::builder(&json.identifier, &json.dockerfile);
    if let Some(ctx) = &json.build_context {
        builder = builder.with_build_context(ctx);
    }
    if let Some(tag) = &json.image_tag {
        builder = builder.with_image_tag(tag);
    }
    if let Some(n) = &json.network {
        builder = builder.with_network(n);
    }
    if let Some(env_vars) = &json.env_vars {
        for (k, v) in env_vars {
            builder = builder.with_env_var(k, v);
        }
    }
    if let Some(runtime_args) = &json.runtime_args {
        let order = ["web_app_port", "postgres_connection_string", "kafka_bootstrap"];
        for key in order {
            if let Some(v) = runtime_args.get(key) {
                builder = builder.with_runtime_arg(key, v);
            }
        }
        for (k, v) in runtime_args {
            if !order.contains(&k.as_str()) {
                builder = builder.with_runtime_arg(k, v);
            }
        }
    }
    if let Some(mappings) = &json.port_mappings {
        for m in mappings {
            builder = builder.with_port_mapping(m.host_port, m.container_port);
        }
    }
    builder = apply_readiness_checks_to_container(builder, &collect_container_readiness(json));
    Box::new(builder.build().await)
}

fn build_exec_component(json: &ExecJson) -> Component {
    let mut builder = ExecutableComponent::builder(&json.identifier)
        .with_executable_path(&json.executable_path);

    if let Some(source_path) = &json.source_path {
        builder = builder.with_source_path(source_path);
    }
    if let Some(build_tool) = &json.build_tool {
        builder = builder.with_build_tool(build_tool_from_json(build_tool));
    }
    if let Some(env_vars) = &json.env_vars {
        for (k, v) in env_vars {
            builder = builder.with_env_var(k, v);
        }
    }
    if let Some(runtime_args) = &json.runtime_args {
        let order = ["web_app_port", "postgres_connection_string", "kafka_bootstrap"];
        for key in order {
            if let Some(v) = runtime_args.get(key) {
                builder = builder.with_runtime_arg(key, v);
            }
        }
        for (k, v) in runtime_args {
            if !order.contains(&k.as_str()) {
                builder = builder.with_runtime_arg(k, v);
            }
        }
    }
    builder = apply_readiness_checks_to_exec(builder, &collect_exec_readiness(json));

    Box::new(builder.build())
}
