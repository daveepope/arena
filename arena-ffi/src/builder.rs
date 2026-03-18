use arena::{Component, Dependency, Encounter, EncounterTrait};
use arena_executable_component::executable_component::ExecutableComponent;
use arena_executable_component::BuildTool;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;

use crate::parse::{BuildToolJson, ComponentJson, DependencyJson, EncounterJson, ExecJson};

const DEFAULT_NETWORK: &str = "arena-network";
const DEFAULT_ENCOUNTER_NAME: &str = "arena-encounter";

pub(super) fn build_encounter(json: &EncounterJson) -> Box<dyn EncounterTrait> {
    let network = json.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    let encounter_name = json.encounter_name.as_deref().unwrap_or(DEFAULT_ENCOUNTER_NAME);
    let dependencies = build_dependencies(json, network);
    let components = build_components(json);
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
            let dep = PostgresDependency::builder(&p.identifier)
                .with_image(p.image.as_deref().unwrap_or("14.20-trixie"))
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
            for topic in k.topics.as_deref().unwrap_or(&[]) {
                builder = builder.with_topic(topic);
            }
            let dep = builder.build();
            Some(Box::new(dep))
        }
    }
}

fn build_components(json: &EncounterJson) -> Vec<Component> {
    let comps = json.components.as_deref().unwrap_or(&[]);
    comps.iter().filter_map(build_component).collect()
}

fn build_component(json: &ComponentJson) -> Option<Component> {
    match json {
        ComponentJson::Exec(e) => Some(build_exec_component(e)),
    }
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

    Box::new(builder.build())
}
