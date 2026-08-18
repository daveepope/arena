load("@rules_java//java:defs.bzl", "java_library")

def arena_junit_classifier_native_lib(name, classifier, resources, classifier_artifacts):
    java_library(
        name = name,
        resources = resources,
        visibility = ["//tools/predeploy_smoke/maven:__pkg__"],
    )
    classifier_artifacts[classifier] = ":" + name
