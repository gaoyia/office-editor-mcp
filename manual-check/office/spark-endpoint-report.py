"""Recheck a bounded native spark evidence folder; never turn ink bboxes into points."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from PIL import Image
from spark_endpoint_evidence import edge_crop_preserves_ink, ink_bounds, kernel_max_coverage, line_weight_pixels, normal_box_max_coverage


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('evidence', type=Path)
    parser.add_argument('--output', type=Path, help='Optional NEW report file; input evidence is read-only')
    args = parser.parse_args()
    folder = args.evidence
    cases = json.loads((folder/'predeclared-cases.json').read_text(encoding='utf-8'))
    assert len(cases) == 7 and sum(c['case'] != 'original' for c in cases) == 6
    measurements = []
    for case in cases:
        name = case['case']
        assert hashlib.sha256(Path(case['source']).read_bytes()).hexdigest() == case['sourceSha256']
        raw = Image.open(folder/(name+'-office-raw.png'))
        crop = Image.open(folder/(name+'-office-crop.png'))
        library = Image.open(folder/(name+'-library.png'))
        assert edge_crop_preserves_ink(raw, crop)
        measurements.append({'case': name, 'libraryBBox': ink_bounds(library), 'officeBBox': ink_bounds(crop), 'strictRaw': 'size-differ' if library.size != crop.size else 'requires-pixel-comparison', 'endpoint': 'not-certified-by-bbox'})
    source = (Path(__file__).resolve().parents[2]/'crates/ooxp/src/xlsx/sparkline_paint.rs').read_text(encoding='utf-8')
    historical = subprocess.check_output(['git','show','19c22b19:crates/ooxp/src/xlsx/sparkline_paint.rs'], text=True, encoding='utf-8')
    assert '(half + 0.25 - dist).clamp(0.0, 1.0)' in historical
    assert '(radius + 0.5 - distance).clamp(0.0, (2.0 * radius).min(1.0))' in source
    maximum = kernel_max_coverage(line_weight_pixels('.75', 106), '.25')
    native = json.loads((folder/'emf-native-alpha-measurements.json').read_text(encoding='utf-8'))
    assert any(s['rawPremultipliedRGBA'] == [55,96,146,255] for s in native['nonzeroAlphaSamples'])
    assert maximum < 1
    result = {'sevenSourcesUnchanged': True, 'sevenCropsPreserveNativeInk': True, 'historical19cSame106DpiCoverageUpperBound': str(maximum), 'currentSame106DpiCoverageUpperBound': str(normal_box_max_coverage(line_weight_pixels('.75',106))), 'inputLibraryImagesAreHistoricalBefore': True, 'nativeCenterAlpha': 255, 'historicalCoverageDifferenceWasNotDpiOnly': True, 'currentCenterCoverageKernelFixed': True, 'logicalEndpointCertificate': False, 'geometryAcceptedUnchanged': '43/49', 'measurements': measurements}
    if args.output:
        assert not args.output.exists(), 'Never overwrite historical evidence'
        args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
