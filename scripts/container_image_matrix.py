from __future__ import annotations

import json
import sys
from pathlib import Path

from container_defaults import image_refs


def main() -> int:
    refs = image_refs(Path(__file__).resolve().parent.parent)
    json.dump(refs, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
