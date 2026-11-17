import copy
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile
from geometry_fixture import NS, CNS, N, merge_chart_seed, used_dimension, rewrite_styles, validate_fixture
STYLE = f'<styleSheet xmlns="{NS}"><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><cellStyleXfs count="1"><xf fontId="0"/></cellStyleXfs><cellXfs count="1"><xf fontId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" builtinId="0" xfId="0"/></cellStyles></styleSheet>'
def sheet():
    s=ET.Element(f"{{{NS}}}worksheet");ET.SubElement(s,f"{{{NS}}}dimension",{"ref":"A20:B23"});sd=ET.SubElement(s,f"{{{NS}}}sheetData")
    for ri,values in [(20,[("B20","S")]),(21,[("A21","A"),("B21",1)]),(22,[("A22","B"),("B22",4)]),(23,[("A23","C"),("B23",2)])]:
        r=ET.SubElement(sd,f"{{{NS}}}row",{"r":str(ri)})
        for address,v in values:
            c=ET.SubElement(r,f"{{{NS}}}c",{"r":address})
            if isinstance(v,str):
                c.set("t","inlineStr");ET.SubElement(ET.SubElement(c,f"{{{NS}}}is"),f"{{{NS}}}t").text=v
            else:ET.SubElement(c,f"{{{NS}}}v").text=str(v)
    return s
def chart():
    c=ET.Element(f"{{{CNS}}}chartSpace")
    for num,formula,values in [(False,"Sheet1!$B$20",["S"]),(False,"Sheet1!$A$21:$A$23",["A","B","C"]),(True,"Sheet1!$B$21:$B$23",[1,4,2])]:
        r=ET.SubElement(c,f"{{{CNS}}}"+("numRef" if num else "strRef"));ET.SubElement(r,f"{{{CNS}}}f").text=formula
        cache=ET.SubElement(r,f"{{{CNS}}}"+("numCache" if num else "strCache"));ET.SubElement(cache,f"{{{CNS}}}ptCount",{"val":str(len(values))})
        for i,v in enumerate(values):ET.SubElement(ET.SubElement(cache,f"{{{CNS}}}pt",{"idx":str(i)}),f"{{{CNS}}}v").text=str(v)
    return c
class GeometryFixtureTests(unittest.TestCase):
    def book(self,s=None,c=None,st=None):
        t=tempfile.TemporaryDirectory();self.addCleanup(t.cleanup);p=Path(t.name)/"test.xlsx"
        with zipfile.ZipFile(p,"w") as z:
            z.writestr("xl/worksheets/sheet1.xml",ET.tostring(s if s is not None else sheet()));z.writestr("xl/charts/chart1.xml",ET.tostring(c if c is not None else chart()));z.writestr("xl/styles.xml",st if st is not None else STYLE)
        return p
    def test_actual_formula_cache_consistent(self):self.assertEqual(len(validate_fixture(self.book())["chartReferences"]),3)
    def test_stale_cache_rejected(self):
        c=chart();c.find(".//c:numCache/c:pt/c:v",N).text="999"
        with self.assertRaisesRegex(AssertionError,"cache disagrees"):validate_fixture(self.book(c=c))
    def test_missing_chart_data_rejected(self):
        s=sheet();sd=s.find("m:sheetData",N);sd.remove(sd[-1]);s.find("m:dimension",N).set("ref",used_dimension(sd))
        with self.assertRaisesRegex(AssertionError,"missing chart cell"):validate_fixture(self.book(s=s))
    def test_wrong_dimension_rejected(self):
        s=sheet();s.find("m:dimension",N).set("ref","A1:D1")
        with self.assertRaisesRegex(AssertionError,"dimension"):validate_fixture(self.book(s=s))
    def test_merge_preserves_seed_rows_and_non_a1(self):
        s=sheet();sd=ET.Element(f"{{{NS}}}sheetData");r=ET.SubElement(sd,f"{{{NS}}}row",{"r":"2"})
        for a in ["B2","C2","D2","E2"]:ET.SubElement(r,f"{{{NS}}}c",{"r":a})
        merge_chart_seed(s,sd);self.assertEqual(used_dimension(sd),"A2:E23");self.assertEqual([r.get("r") for r in sd],["2","20","21","22","23"])
        s.remove(s.find("m:sheetData",N));s.append(sd);s.find("m:dimension",N).set("ref",used_dimension(sd));self.assertEqual(len(validate_fixture(self.book(s=s))["chartReferences"]),3)
    def test_independent_normal_variants(self):
        for v,xf,nf,cf in [(False,0,"Calibri","Calibri"),("name-only",0,"Calibri","Calibri"),("xf-only",1,"Calibri","Calibri"),("font-only",0,"Arial","Calibri"),("cell-only",0,"Calibri","Arial"),(True,1,"Arial","Calibri")]:
            with self.subTest(variant=v):
                r=validate_fixture(self.book(st=rewrite_styles(STYLE,"Calibri",11,v)));self.assertEqual((r["normalXf"],r["normalFont"]["name"],r["cellXf0Font"]["name"]),(xf,nf,cf))
    def test_invalid_normal_xf_rejected(self):
        st=ET.fromstring(STYLE);st.find("m:cellStyles/m:cellStyle",N).set("xfId","9")
        with self.assertRaises(AssertionError):validate_fixture(self.book(st=ET.tostring(st)))
    def test_report_display_keeps_raw_alpha_in_strict_comparison(self):
        import csv
        import subprocess
        import sys
        from PIL import Image
        with tempfile.TemporaryDirectory() as tmp:
            d=Path(tmp);base={"case":"alpha","action":"chart","width":"2","height":"2","mdw96":"7","widthMode":"explicit","hidden":"False","error":"","workbookSha256":"same"}
            for name,rows in [("before.csv",[base]),("after.csv",[base]),("oracle.csv",[{"case":"alpha","action":"chart","goldWidth":"2","goldHeight":"2","error":""}])]:
                with (d/name).open("w",newline="",encoding="utf-8") as f:
                    w=csv.DictWriter(f,rows[0]);w.writeheader();w.writerows(rows)
            for phase in ["before","after"]:Image.new("RGB",(2,2),"white").save(d/f"alpha-chart-{phase}.png")
            gold=d/"alpha-chart-gold.png";Image.new("RGBA",(2,2),(0,0,0,0)).save(gold);original=gold.read_bytes()
            subprocess.run([sys.executable,str(Path(__file__).with_name("geometry-report.py")),str(d)],check=True,capture_output=True)
            self.assertEqual(gold.read_bytes(),original)
            self.assertEqual(Image.open(d/"alpha-chart-compare.png").getpixel((49,30)),(255,255,255,255))
            with (d/"metrics.csv").open(encoding="utf-8-sig") as f:row=next(csv.DictReader(f))
            self.assertEqual(row["strictResult"],"pixelDiffer");self.assertEqual(row["differPixels"],"4")

if __name__=="__main__":unittest.main()
