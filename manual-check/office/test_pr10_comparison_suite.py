import json
from pathlib import Path
import tempfile
import unittest
import zipfile
from pr10_comparison_suite import (DRAWING, MAIN, check_package,
    drawing_style_errors, prepare_source, repair_optional_empty_style, size_status, read_request, request_signature)


class ComparisonSuiteTests(unittest.TestCase):
    def xml(self, style='<xdr:style/>'):
        return (f'<xdr:wsDr xmlns:xdr="{DRAWING}" xmlns:a="{MAIN}">'
                '<xdr:absoluteAnchor><xdr:sp><xdr:nvSpPr/><xdr:spPr/>'
                f'{style}</xdr:sp></xdr:absoluteAnchor></xdr:wsDr>').encode()

    def test_windows_path_spelling_does_not_create_false_ambiguity(self):
        self.assertEqual(request_signature({"path": "D:\\work\\a.xlsx", "action": "exportSheetImage"}),
                         request_signature({"path": "D:/work/a.xlsx", "action": "exportSheetImage"}))
        self.assertNotEqual(request_signature({"path": "a.xlsx", "scale": 1}),
                            request_signature({"path": "a.xlsx", "scale": 2}))

    def test_request_bom_is_explicit_not_ansi_guessing(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/"request.json"
            p.write_text('{"action":"exportSheetImage"}',encoding="utf-16")
            self.assertEqual(read_request(p)["action"],"exportSheetImage")

    def test_actual_stroke_generator_omits_invalid_optional_style(self):
        text=(Path(__file__).parent/"cases"/"xlsx-sheet-stroke-image.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("strokeHollow",text)
        self.assertNotIn("<xdr:style/>",text)

    def test_empty_style_violates_all_four_required_refs(self):
        self.assertEqual(len(drawing_style_errors(self.xml())), 1)
        self.assertEqual(drawing_style_errors(self.xml())[0]['required'],
                         ['lnRef', 'fillRef', 'effectRef', 'fontRef'])

    def test_omit_optional_style_preserving_all_other_bytes(self):
        raw = self.xml()
        changed, count = repair_optional_empty_style(raw)
        self.assertEqual(count, 1)
        self.assertEqual(changed, raw.replace(b'<xdr:style/>', b''))
        self.assertEqual(drawing_style_errors(changed), [])

    def test_keep_valid_style_and_explicit_properties(self):
        style = '<xdr:style>' + ''.join(f'<a:{n}/>' for n in ('lnRef','fillRef','effectRef','fontRef')) + '</xdr:style>'
        raw = self.xml(style)
        self.assertEqual(repair_optional_empty_style(raw), (raw, 0))
        self.assertEqual(drawing_style_errors(raw), [])

    def test_do_not_swallow_nonempty_invalid_style(self):
        raw = self.xml('<xdr:style><a:lnRef/></xdr:style>')
        self.assertEqual(repair_optional_empty_style(raw), (raw, 0))
        self.assertTrue(drawing_style_errors(raw))

    def test_fixture_copy_changes_only_drawing_part(self):
        with tempfile.TemporaryDirectory() as td:
            a,b = Path(td)/'old.xlsx',Path(td)/'fixed.xlsx'
            with zipfile.ZipFile(a,'w') as z:
                z.writestr('xl/drawings/drawing1.xml',self.xml())
                z.writestr('unchanged.bin',b'\x00\xffpayload')
            before = a.read_bytes(); r=prepare_source(a,b)
            self.assertEqual(a.read_bytes(),before)
            self.assertEqual(r['changedParts'],['xl/drawings/drawing1.xml'])
            with zipfile.ZipFile(b) as z:self.assertEqual(z.read('unchanged.bin'),b'\x00\xffpayload')
            self.assertEqual(r['staticErrors'],[])
            self.assertFalse(r['officeVerified'])

    def test_destination_overwrite_is_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'test.xlsx';p.write_bytes(b'preserve')
            with self.assertRaises(ValueError):prepare_source(p,p)
            self.assertEqual(p.read_bytes(),b'preserve')

    def test_size_differ_none_never_means_pass(self):
        s=size_status((217,45),(65,161))
        self.assertFalse(s['nativeSameSize']);self.assertIsNone(s['pixelDiffer'])
        self.assertEqual(s['pixelStatus'],'NOT_COMPUTED_SIZE_MISMATCH')
        self.assertFalse(s['accepted'])

    def test_same_size_also_requires_content_review(self):
        s=size_status((65,161),(65,161))
        self.assertTrue(s['nativeSameSize']);self.assertFalse(s['accepted'])

    def test_missing_relationship_is_static_failure(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'test.xlsx'
            with zipfile.ZipFile(p,'w') as z:z.writestr('_rels/.rels','<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Target="missing.xml"/></Relationships>')
            self.assertEqual(check_package(p)[0]['code'],'missing-relationship-target')


if __name__ == '__main__':
    unittest.main()
