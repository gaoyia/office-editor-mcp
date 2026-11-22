"""Namespace-preserving, new-output-only controlled chart fixture derivation.

This is NOT an Office exporter or a renderer. The existing read-only package
preflight is applied before transformation and to every transformed XML part.
DOM retains namespace declarations used only by MC QName/token attributes;
ElementTree's namespace reserialization is not safe for that use case.
"""
import hashlib
import importlib.util
from pathlib import Path
import re
import zipfile
from xml.dom import Node, minidom

_spec = importlib.util.spec_from_file_location(
    "controlled_fixture_preflight", Path(__file__).parents[1] / "validate-controlled-chart-package.py")
_preflight = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_preflight)
CHART = "http://schemas.openxmlformats.org/drawingml/2006/chart"


def remove_chart_legend(data):
    """Remove exactly one direct c:legend; retain other nodes and bindings."""
    _preflight.check_xml(data)
    with minidom.parseString(data) as document:
        if document.doctype is not None:
            raise ValueError("Controlled fixture must not contain a DOCTYPE")
        root = document.documentElement
        if root.namespaceURI != CHART or root.localName != "chartSpace":
            raise ValueError("Expected DrawingML chartSpace")
        direct = lambda parent, name: [n for n in parent.childNodes
            if n.nodeType == Node.ELEMENT_NODE and n.namespaceURI == CHART and n.localName == name]
        charts = direct(root, "chart")
        if len(charts) != 1:
            raise ValueError("Expected exactly one direct chart")
        legends = direct(charts[0], "legend")
        if len(legends) != 1:
            raise ValueError("Expected exactly one direct legend")
        charts[0].removeChild(legends[0]).unlink()
        result = document.toxml(encoding="utf-8")
    _preflight.check_xml(result)
    return result


def derive_without_legends(source, output, expected_source_sha):
    """Clone a validated package, changing only chart XML legend nodes.

    No overwrite, source modification, Office/COM, binding or guard changes.
    A failed partial output is retained for diagnosis, never reused as gold.
    """
    source, output = Path(source), Path(output)
    if source.resolve() == output.resolve() or output.exists():
        raise FileExistsError("Require a distinct new output")
    before = source.read_bytes()
    sha = lambda b: hashlib.sha256(b).hexdigest()
    if sha(before).lower() != expected_source_sha.lower():
        raise ValueError("Source SHA mismatch")
    _preflight.check_package(source)
    with zipfile.ZipFile(source) as package:
        archive_comment = package.comment
        infos = package.infolist()
        if len({i.filename for i in infos}) != len(infos):
            raise ValueError("Duplicate package part names")
        entries, changed = [], []
        for info in infos:
            data = package.read(info)
            if re.fullmatch(r"xl/charts/chart\d+\.xml", info.filename):
                data = remove_chart_legend(data)
                changed.append(info.filename)
            if info.filename.endswith(".xml"):
                _preflight.check_xml(data)
            entries.append((info, data))
        if not changed:
            raise ValueError("No chart parts selected")
    # Do all validation before creating a destination. ZipFile mode x is the
    # final race-safe no-overwrite gate, not just an earlier exists check.
    if source.read_bytes() != before:
        raise ValueError("Source changed during derivation")
    with zipfile.ZipFile(output, "x") as package:
        package.comment = archive_comment
        for info, data in entries:
            package.writestr(info, data)
    count = _preflight.check_package(output)
    if source.read_bytes() != before:
        raise ValueError("Source changed during derivation")
    return {"Status": "DERIVED_FIXTURE_ONLY_NOT_OFFICE_GOLD",
            "SourceSha": sha(before), "OutputSha": sha(output.read_bytes()),
            "ChangedParts": changed, "XmlPartsChecked": count,
            "Transformation": "Remove exactly one direct chart legend per chart; retain namespace bindings and all other nodes",
            "OfficeCalled": False, "SourceUnchanged": True}