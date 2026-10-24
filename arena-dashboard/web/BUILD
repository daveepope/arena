load("@bazel_skylib//rules:copy_file.bzl", "copy_file")
load("@aspect_rules_js//js:defs.bzl", "js_run_binary")
load("@aspect_rules_ts//ts:defs.bzl", "ts_project")
load("@npm//:defs.bzl", "npm_link_all_packages")
load("@npm//arena-dashboard/web:@playwright/test/package_json.bzl", playwright_bin = "bin")
load("@npm//arena-dashboard/web:@tailwindcss/cli/package_json.bzl", tailwindcss_bin = "bin")
load("@npm//arena-dashboard/web:vite/package_json.bzl", vite_bin = "bin")
load("@npm//arena-dashboard/web:vitest/package_json.bzl", vitest_bin = "bin")

npm_link_all_packages(name = "node_modules")

copy_file(
    name = "index_html_for_embed",
    src = "index.html",
    out = "embed/index.html",
    visibility = ["//arena-dashboard:__pkg__"],
)

tailwindcss_bin.tailwindcss_binary(name = "tailwindcss_cli")

vite_bin.vite_binary(name = "vite_cli")

vitest_bin.vitest_binary(name = "vitest_cli")

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

ts_project(
    name = "ts_browser",
    srcs = glob(["browser/**/*.ts"]) + ["playwright.config.ts"],
    extends = "tsconfig.json",
    no_emit = True,
    tsconfig = "browser/tsconfig.json",
    deps = [
        ":node_modules/@playwright/test",
        ":node_modules/@types/node",
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
    visibility = ["//arena-dashboard:__pkg__"],
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
    visibility = ["//arena-dashboard:__pkg__"],
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
        ":node_modules/@vitest/coverage-v8",
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

js_run_binary(
    name = "coverage_lcov_raw",
    srcs = glob([
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
        ":node_modules/@vitest/coverage-v8",
        ":node_modules/@xyflow/react",
        ":node_modules/dagre",
        ":node_modules/jsdom",
        ":node_modules/react",
        ":node_modules/react-dom",
        ":node_modules/vite",
        ":node_modules/vitest",
        ":node_modules/zustand",
    ],
    outs = ["coverage-raw/lcov.info"],
    tool = ":vitest_cli",
    chdir = package_name(),
    args = [
        "run",
        "--config",
        "vite.config.ts",
        "--coverage",
    ],
    tags = ["manual"],
    visibility = ["//visibility:public"],
)

genrule(
    name = "coverage_lcov",
    srcs = [":coverage_lcov_raw"],
    outs = ["coverage/lcov.info"],
    cmd = "$(execpath //tools/lcov_source_paths:prefix_source_paths) " +
          package_name() + " $< $@",
    tools = ["//tools/lcov_source_paths:prefix_source_paths"],
    tags = ["manual"],
    visibility = ["//visibility:public"],
)

config_setting(
    name = "linux_x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "macos_x86_64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "macos_aarch64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:aarch64",
    ],
)

_CHROME_FILES = select({
    ":linux_x86_64": [
        "@chrome_for_testing_linux_x86_64//:chrome",
        "@chrome_for_testing_linux_x86_64//:chrome_files",
    ],
    ":macos_x86_64": [
        "@chrome_for_testing_macos_x86_64//:chrome",
        "@chrome_for_testing_macos_x86_64//:chrome_files",
    ],
    ":macos_aarch64": [
        "@chrome_for_testing_macos_aarch64//:chrome",
        "@chrome_for_testing_macos_aarch64//:chrome_files",
    ],
    "//conditions:default": [],
})

_BROWSER_SPEC_DATA = [
    "playwright.config.ts",
    "tsconfig.json",
    "browser/tsconfig.json",
    ":node_modules/@playwright/test",
    "//arena-dashboard:demo_lifecycle",
] + glob(["browser/**/*.ts"]) + _CHROME_FILES

_BROWSER_SPEC_ENV = {
    "ARENA_DEMO_LIFECYCLE_BIN": "$(rlocationpath //arena-dashboard:demo_lifecycle)",
} | select({
    ":linux_x86_64": {
        "ARENA_CHROME_BINARY": "$(rlocationpath @chrome_for_testing_linux_x86_64//:chrome)",
    },
    ":macos_x86_64": {
        "ARENA_CHROME_BINARY": "$(rlocationpath @chrome_for_testing_macos_x86_64//:chrome)",
    },
    ":macos_aarch64": {
        "ARENA_CHROME_BINARY": "$(rlocationpath @chrome_for_testing_macos_aarch64//:chrome)",
    },
    "//conditions:default": {},
})

_BROWSER_TEST_DATA = _BROWSER_SPEC_DATA + ["//arena-dashboard:server"]

_BROWSER_TEST_ENV = _BROWSER_SPEC_ENV | {
    "ARENA_DASHBOARD_SERVER_BIN": "$(rlocationpath //arena-dashboard:server)",
}

_BROWSER_TEST_NO_COPY_TO_BIN = _CHROME_FILES

_BROWSER_TEST_PLATFORM = select({
    ":linux_x86_64": [],
    ":macos_x86_64": [],
    ":macos_aarch64": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

playwright_bin.playwright_test(
    name = "browser_test",
    size = "small",
    args = [
        "test",
        "--config",
        "playwright.config.ts",
    ],
    chdir = package_name(),
    data = _BROWSER_TEST_DATA,
    env = _BROWSER_TEST_ENV,
    no_copy_to_bin = _BROWSER_TEST_NO_COPY_TO_BIN,
    tags = [
        "component_test",
        "cpu_profile_test",
    ],
    target_compatible_with = _BROWSER_TEST_PLATFORM,
)

playwright_bin.playwright_test(
    name = "browser_test_staged_server",
    size = "small",
    args = [
        "test",
        "--config",
        "playwright.config.ts",
    ],
    chdir = package_name(),
    data = _BROWSER_SPEC_DATA,
    env = _BROWSER_SPEC_ENV,
    no_copy_to_bin = _BROWSER_TEST_NO_COPY_TO_BIN,
    tags = [
        "component_test",
        "manual",
        "no-sandbox",
    ],
    target_compatible_with = _BROWSER_TEST_PLATFORM,
)

playwright_bin.playwright_test(
    name = "browser_test_headed",
    size = "small",
    args = [
        "test",
        "--config",
        "playwright.config.ts",
        "--headed",
    ],
    chdir = package_name(),
    data = _BROWSER_TEST_DATA,
    env = _BROWSER_TEST_ENV,
    no_copy_to_bin = _BROWSER_TEST_NO_COPY_TO_BIN,
    tags = [
        "component_test",
        "manual",
    ],
    target_compatible_with = _BROWSER_TEST_PLATFORM,
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
