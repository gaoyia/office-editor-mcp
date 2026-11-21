"""Create an MC-preserving nolegend control; never invokes Office or overwrites."""
import argparse
import json
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent / "lib"))
from controlled_chart_fixture import derive_without_legends

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source")
    parser.add_argument("output")
    parser.add_argument("--expected-source-sha", required=True)
    args = parser.parse_args()
    try:
        result = derive_without_legends(args.source, args.output, args.expected_source_sha)
    except (OSError, ValueError) as error:
        print(json.dumps({"Status": "REJECTED_BEFORE_OFFICE", "Error": str(error)}))
        return 1
    print(json.dumps(result))
    return 0
if __name__ == "__main__":
    raise SystemExit(main())