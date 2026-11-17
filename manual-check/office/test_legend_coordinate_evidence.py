import copy
import unittest
from legend_coordinate_evidence import worksheet_area_local, translation_certifies_chart_local, nominal_point_rect, legend_without_overlay

class LegendCoordinateEvidenceTests(unittest.TestCase):
    def samples(self):
        a={'chartObjectWorksheetRect':{'left':195.6,'top':14.2,'width':432,'height':252},'legendRect':{'left':212,'top':21.2,'width':172.8,'height':75.6},'keys':[{'left':258.9,'top':55.7}]}
        b=copy.deepcopy(a);b['chartObjectWorksheetRect']['left']+=72;b['chartObjectWorksheetRect']['top']+=36
        return a,b
    def test_same_chart_translation_proves_local_not_worksheet_coordinates(self):
        a,b=self.samples();self.assertTrue(translation_certifies_chart_local(a,b,True))
    def test_moved_legend_or_changed_native_export_is_not_certified(self):
        a,b=self.samples();self.assertFalse(translation_certifies_chart_local(a,b,False));b['legendRect']['left']+=72;self.assertFalse(translation_certifies_chart_local(a,b,True))
    def test_chart_area_worksheet_origin_is_subtracted_not_added_to_legend(self):
        a,b=self.samples();area={'left':267.6,'top':50.2,'width':426,'height':246};local=worksheet_area_local(area,b['chartObjectWorksheetRect']);self.assertAlmostEqual(local['left'],0);self.assertAlmostEqual(local['top'],0);self.assertEqual(local['width'],426)
    def test_point_scaling_does_not_claim_png_origin_or_add_four_points(self):
        r=nominal_point_rect({'left':212,'top':21.2,'width':172.8,'height':75.6},106);self.assertEqual(r['left'],212*106/72);self.assertFalse(r['pngOriginCalibrated'])
    def test_overlay_only_comparison_does_not_erase_legal_style_or_manual_config(self):
        a='<legend><legendPos val="r"/><overlay val="0"/></legend>';b=a.replace('val="0"','val="1"');self.assertEqual(legend_without_overlay(a),legend_without_overlay(b));self.assertNotEqual(legend_without_overlay(a),legend_without_overlay(b.replace('</legend>','<txPr sz="1800"/></legend>')))
    def test_invalid_dpi_is_not_a_coordinate_certificate(self):
        with self.assertRaises(ValueError):nominal_point_rect({'left':0,'top':0,'width':1,'height':1},0)

if __name__=='__main__':unittest.main()
