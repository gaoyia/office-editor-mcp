from pathlib import Path
import hashlib,json,tempfile,unittest
from comparison_outcomes import qualify,collect

class ReferenceQualificationTests(unittest.TestCase):
 def fixture(self):
  t=tempfile.TemporaryDirectory();self.addCleanup(t.cleanup);root=Path(t.name);s=root/'source.xlsx';s.write_bytes(b'local synthetic fixture');p=root/'raw.png';p.write_bytes(b'local synthetic encoded image')
  sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest();h=sha(s)
  e={'entry':'fixture','scope':'main','prepared':{'sourceSha256':h},'request':{'action':'exportChartImage','id':'sheet:Sheet1/chart:1'}}
  c={'status':'CLEAN_PUBLIC_OFFICE_EXPORT','source':str(s),'output':str(p),'sourceShaBefore':h,'sourceShaAfter':h,'sourceUnchanged':True,'exclusiveRead':True,'ownWorkbookClosed':True,'outputSha256':sha(p),'id':e['request']['id']}
  return root,e,c
 def test_clean_source_bound_reference(self):
  r,e,c=self.fixture();self.assertTrue(qualify(c,e,r)[0])
 def test_repair_is_never_gold(self):
  r,e,c=self.fixture();c['repairLogs']=['own-recovery.xml'];self.assertFalse(qualify(c,e,r)[0])
 def test_closed_and_hash_stable_required(self):
  r,e,c=self.fixture();c['exclusiveRead']=False;self.assertFalse(qualify(c,e,r)[0])
 def test_current_image_hash_checked(self):
  r,e,c=self.fixture();Path(c['output']).write_bytes(b'changed');self.assertFalse(qualify(c,e,r)[0])
 def test_wrong_chart_rejected(self):
  r,e,c=self.fixture();c['id']='sheet:Sheet1/chart:2';self.assertFalse(qualify(c,e,r)[0])
 def test_unqualified_vector_not_promoted(self):
  r,e,c=self.fixture();c['referenceQualified']=False;self.assertFalse(qualify(c,e,r)[0])
 def test_word_missing_reference_is_explicit_blocked_not_renderer_ni(self):
  r,e,c=self.fixture();e['request']['action']='exportPageImage';e['request']['id']='page:1';rows=collect(r,{'entries':[e]});self.assertEqual(rows[0]['status'],'BLOCKED_WORD_ORACLE_UNQUALIFIED');self.assertFalse(rows[0]['qualified'])
 def test_historical_record_not_automatically_reexported(self):
  r,e,c=self.fixture();e['scope']='historical-blocker-reference-only';self.assertEqual(collect(r,{'entries':[e]})[0]['status'],'HISTORICAL_REFERENCE_ONLY')
 def test_scripts_have_no_desktop_output_or_example_dependency(self):
  base=Path(__file__).parent
  names=['comparison_outcomes.py','pr10_comparison_suite.py','comparison-suite-report.py','comparison-oracle.ps1','comparison-excel-batch.ps1','comparison-excel-targets.ps1','comparison-powerpoint-oracle.ps1']
  for n in names:
   s=(base/n).read_text(encoding='utf-8-sig');self.assertNotIn('C:\\Users\\',s);self.assertNotIn('GetFolderPath',s);self.assertNotIn('C:/Users/',s)

if __name__=='__main__':unittest.main()
