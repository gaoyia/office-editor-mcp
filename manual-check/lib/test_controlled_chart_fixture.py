import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
import hashlib
from xml.dom import minidom
spec=importlib.util.spec_from_file_location("fixture",Path(__file__).with_name("controlled_chart_fixture.py"))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
C=m.CHART
XML=f'''<c:chartSpace xmlns:c="{C}" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:c14="urn:c14" mc:Ignorable="c14"><c:chart><c:title/><c:legend><c:legendPos val="r"/></c:legend><mc:AlternateContent><mc:Choice Requires="c14"><c14:payload/></mc:Choice></mc:AlternateContent><c:plotArea/></c:chart></c:chartSpace>'''.encode()
class FixtureTests(unittest.TestCase):
    def test_namespace_tokens_and_all_other_nodes_preserved(self):
        result=m.remove_chart_legend(XML)
        self.assertIn(b'xmlns:c14="urn:c14"',result);self.assertIn(b'Requires="c14"',result);self.assertIn(b'mc:Ignorable="c14"',result)
        with minidom.parseString(XML) as a,minidom.parseString(result) as b:
            legend=a.getElementsByTagNameNS(C,"legend")[0];legend.parentNode.removeChild(legend)
            self.assertEqual(a.toxml(),b.toxml());self.assertEqual(len(b.getElementsByTagNameNS(C,"legend")),0)
    def test_locally_scoped_qname_and_unused_declaration_retained(self):
        data=XML.replace(b'xmlns:c14="urn:c14"',b'xmlns:c14="urn:c14" xmlns:spare="urn:unused"')
        result=m.remove_chart_legend(data);self.assertIn(b'xmlns:spare="urn:unused"',result);m._preflight.check_xml(result)
    def test_namespace_used_only_in_mc_attribute_tokens_retained(self):
        data=XML.replace(b'<c14:payload/>',b'<c:payload/>')
        result=m.remove_chart_legend(data)
        self.assertIn(b'xmlns:c14="urn:c14"',result)
        self.assertIn(b'Requires="c14"',result)
        m._preflight.check_xml(result)
    def test_archive_comment_and_entry_metadata_preserved(self):
        with tempfile.TemporaryDirectory() as d:
            a=Path(d)/'a.xlsx';b=Path(d)/'b.xlsx'
            info=zipfile.ZipInfo('xl/charts/chart1.xml',(2020,1,2,3,4,6));info.comment=b'part comment'
            with zipfile.ZipFile(a,'x') as z:
                z.comment=b'archive comment';z.writestr(info,XML)
            m.derive_without_legends(a,b,hashlib.sha256(a.read_bytes()).hexdigest())
            with zipfile.ZipFile(b) as z:
                self.assertEqual(z.comment,b'archive comment');self.assertEqual(z.getinfo(info.filename).date_time,info.date_time);self.assertEqual(z.getinfo(info.filename).comment,info.comment)
    def test_reject_unbound_mc_without_repair_or_bypass(self):
        with self.assertRaisesRegex(ValueError,'Unbound'):m.remove_chart_legend(XML.replace(b' xmlns:c14="urn:c14"',b''))
    def test_require_single_direct_chart_and_legend(self):
        for data in [b'<r/>', XML.replace(b'<c:legend>',b'<c:legend/><c:legend>'),XML.replace(b'<c:legend><c:legendPos val="r"/></c:legend>',b'')]:
            with self.assertRaises(ValueError):m.remove_chart_legend(data)
    def test_package_source_other_parts_and_new_output_only(self):
        with tempfile.TemporaryDirectory() as d:
            a=Path(d)/'source.xlsx';b=Path(d)/'new.xlsx'
            with zipfile.ZipFile(a,'x') as z:z.writestr('xl/charts/chart1.xml',XML);z.writestr('untouched.bin',b'fixed')
            before=a.read_bytes();h=hashlib.sha256(before).hexdigest();rec=m.derive_without_legends(a,b,h)
            self.assertEqual(a.read_bytes(),before);self.assertEqual(rec['ChangedParts'],['xl/charts/chart1.xml']);self.assertFalse(rec['OfficeCalled'])
            with zipfile.ZipFile(b) as z:self.assertEqual(z.read('untouched.bin'),b'fixed')
            old=b.read_bytes()
            with self.assertRaises(FileExistsError):m.derive_without_legends(a,b,h)
            with self.assertRaises(FileExistsError):m.derive_without_legends(a,a,h)
            self.assertEqual(b.read_bytes(),old)
    def test_wrong_sha_or_invalid_xml_creates_no_output(self):
        with tempfile.TemporaryDirectory() as d:
            a=Path(d)/'a.xlsx';b=Path(d)/'b.xlsx'
            with zipfile.ZipFile(a,'x') as z:z.writestr('xl/charts/chart1.xml',XML)
            with self.assertRaises(ValueError):m.derive_without_legends(a,b,'0'*64)
            self.assertFalse(b.exists())
            bad=Path(d)/'bad.xlsx'
            with zipfile.ZipFile(bad,'x') as z:z.writestr('xl/charts/chart1.xml',XML.replace(b' xmlns:c14="urn:c14"',b''))
            with self.assertRaises(ValueError):m.derive_without_legends(bad,b,hashlib.sha256(bad.read_bytes()).hexdigest())
            self.assertFalse(b.exists())
if __name__=='__main__':unittest.main()