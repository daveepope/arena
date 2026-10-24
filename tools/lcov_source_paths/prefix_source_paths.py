import sys


def prefix_source_paths(lcov_text: str, prefix: str) -> str:
    if not prefix:
        return lcov_text
    if not prefix.endswith("/"):
        prefix = prefix + "/"

    prefixed = []
    for line in lcov_text.splitlines():
        if line.startswith("SF:"):
            path = line[len("SF:"):]
            if path and not path.startswith("/") and not path.startswith(prefix):
                line = "SF:" + prefix + path
        prefixed.append(line)

    if not prefixed:
        return ""
    return "\n".join(prefixed) + "\n"


def main(argv: list[str]) -> int:
    _, prefix, source, destination = argv
    with open(source) as source_file:
        text = source_file.read()
    with open(destination, "w") as destination_file:
        destination_file.write(prefix_source_paths(text, prefix))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
