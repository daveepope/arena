load("@aspect_rules_js//js:defs.bzl", "js_run_binary")
load("@aspect_rules_ts//ts:defs.bzl", "ts_project")
load("@npm//:defs.bzl", "npm_link_all_packages")
load("@npm//arena-dashboard/web:@tailwindcss/cli/package_json.bzl", tailwindcss_bin = "bin")
load("@npm//arena-dashboard/web:vite/package_json.bzl", vite_bin = "bin")
load("@npm//arena-dashboard/web:vitest/package_json.bzl", vitest_bin = "bin")

npm_link_all_packages(name = "node_modules")

tailwindcss_bin.tailwindcss_binary(name = "tailwindcss_cli")

vite_bin.vite_binary(name = "vite_cli")

ts_project(
    name = "ts",
    srcs = glob([
        "src/**/*.ts",
        "src/**/*.tsx",
    ]),
    no_emit = True,
    tsconfig = "tsconfig.json",
    deps = [
        ":node_modules/@testing-library/react",
        ":node_modules/@types/dagre",
        ":node_modules/@types/react",
        ":node_modules/@types/react-dom",
        ":node_modules/@xyflow/react",
        ":node_modules/dagre",
        ":node_modules/react",
        ":node_modules/react-dom",
        ":node_modules/vitest",
        ":node_modules/zustand",
    ],
)

js_run_binary(
    name = "bundle",
    srcs = glob([
        "src/**/*.ts",
        "src/**/*.tsx",
    ]) + [
        "tsconfig.json",
        "vite.config.ts",
        ":node_modules/@types/dagre",
        ":node_modules/@types/react",
        ":node_modules/@types/react-dom",
        ":node_modules/@vitejs/plugin-react",
        ":node_modules/@xyflow/react",
        ":node_modules/dagre",
        ":node_modules/react",
        ":node_modules/react-dom",
        ":node_modules/vite",
        ":node_modules/zustand",
    ],
    outs = ["bundle.js"],
    tool = ":vite_cli",
    chdir = package_name(),
    args = ["build", "--config", "vite.config.ts"],
)

js_run_binary(
    name = "css",
    srcs = glob([
        "src/**/*.ts",
        "src/**/*.tsx",
    ]) + [
        "src/styles.css",
        ":node_modules/@xyflow/react",
        ":node_modules/tailwindcss",
    ],
    outs = ["styles.css"],
    tool = ":tailwindcss_cli",
    chdir = package_name(),
    args = [
        "-i",
        "src/styles.css",
        "-o",
        "styles.css",
    ],
    execution_requirements = {"no-sandbox": "1"},
)

vitest_bin.vitest_test(
    name = "vitest_test",
    size = "small",
    args = ["run", "--config", "vite.config.ts"],
    chdir = package_name(),
    data = glob([
        "src/**/*.ts",
        "src/**/*.tsx",
    ]) + [
        "vite.config.ts",
        "tsconfig.json",
        ":node_modules/@testing-library/react",
        ":node_modules/@types/dagre",
        ":node_modules/@types/react",
        ":node_modules/@types/react-dom",
        ":node_modules/@vitejs/plugin-react",
        ":node_modules/@xyflow/react",
        ":node_modules/dagre",
        ":node_modules/jsdom",
        ":node_modules/react",
        ":node_modules/react-dom",
        ":node_modules/vite",
        ":node_modules/vitest",
        ":node_modules/zustand",
    ],
)

filegroup(
    name = "dist",
    srcs = [
        ":bundle",
        ":css",
        "index.html",
    ],
    visibility = ["//arena-dashboard:__pkg__"],
)
