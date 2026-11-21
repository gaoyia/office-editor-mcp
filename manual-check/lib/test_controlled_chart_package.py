import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
spec=importlib.util.spec_from_file_location("preflight",Path(__file__).parents[1]/"validate-controlled-chart-package.py")
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ControlledPackageTests(unittest.TestCase):
    def test_declared_choice(self):
        m.check_xml(b'<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:c14="urn:c14"><mc:Choice Requires="c14"/></r>')
    def test_unbound_choice_rejected(self):
        with self.assertRaises(ValueError):m.check_xml(b'<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="c14"/></r>')
    def test_local_scope_is_resolved(self):
        m.check_xml(b'<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice xmlns:c14="urn:c14" Requires="c14"/></r>')
    def test_sibling_declaration_does_not_leak(self):
        with self.assertRaises(ValueError):m.check_xml(b'<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><a xmlns:c14="urn:c14"/><mc:Choice Requires="c14"/></r>')
    def test_ignorable_prefix_checked(self):
        with self.assertRaises(ValueError):m.check_xml(b'<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="c14"/>')
    def test_read_only_zip_and_no_xml_rejection(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"fixture.xlsx"
            with zipfile.ZipFile(p,"w") as z:z.writestr("xl/charts/chart1.xml",b'<r/>')
            before=p.read_bytes();self.assertEqual(m.check_package(p),1);self.assertEqual(before,p.read_bytes())
            empty=Path(d)/"empty.zip"
            with zipfile.ZipFile(empty,"w"):pass
            with self.assertRaises(ValueError):m.check_package(empty)
    def test_malformed_xml_rejected(self):
        with self.assertRaises(Exception):m.check_xml(b'<r>')
if __name__=="__main__":unittest.main()
