"""Read-only fixture preflight; never opens Office or rewrites a package.

Run before the guarded native exporter. In particular, ElementTree prefix
renaming must not leave mc:Choice/@Requires or mc:Ignorable tokens unbound.
"""
import argparse
import io
import json
import zipfile
import xml.etree.ElementTree as ET
MC = "http://schemas.openxmlformats.org/markup-compatibility/2006"

def check_xml(data):
    scopes = []
    pending = {}
    for event, item in ET.iterparse(io.BytesIO(data), events=("start-ns", "start", "end")):
        if event == "start-ns":
            prefix, uri = item
            pending[prefix or ""] = uri
        elif event == "start":
            scope = dict(scopes[-1]) if scopes else {"xml": "http://www.w3.org/XML/1998/namespace"}
            scope.update(pending)
            pending.clear()
            scopes.append(scope)
            values = []
            if item.tag == "{" + MC + "}Choice":
                values.append(item.get("Requires", ""))
            values.append(item.get("{" + MC + "}Ignorable", ""))
            for value in values:
                for prefix in value.split():
                    if not scope.get(prefix):
                        raise ValueError("Unbound markup-compatibility prefix: " + prefix)
        else:
            scopes.pop()

def check_package(path):
    checked = 0
    with zipfile.ZipFile(path) as package:
        for item in package.infolist():
            if item.filename.endswith(".xml"):
                try:
                    check_xml(package.read(item))
                except (ValueError, ET.ParseError) as error:
                    raise ValueError(item.filename + ": " + str(error)) from error
                checked += 1
    if not checked:
        raise ValueError("No XML parts")
    return checked

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package")
    args = parser.parse_args()
    try:
        count = check_package(args.package)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(json.dumps({"status": "REJECTED_BEFORE_OFFICE", "error": str(error)}))
        return 1
    print(json.dumps({"status": "XML_MC_PREFLIGHT_ONLY_NOT_OFFICE_GOLD", "parts": count}))
    return 0
if __name__ == "__main__":
    raise SystemExit(main())
