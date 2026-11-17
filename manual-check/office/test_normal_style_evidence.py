import unittest
from normal_style_evidence import normal_snapshot, same_normal_font, normal_oracle_status
NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
def styles():
    return f'<styleSheet xmlns="{NS}"><fonts><font><name val="Calibri"/><sz val="11"/></font><font><name val="Arial"/><sz val="11"/></font></fonts><cellStyleXfs><xf fontId="0"/><xf fontId="1"/></cellStyleXfs><cellXfs><xf fontId="0" xfId="0"/></cellXfs><cellStyles><cellStyle name="renamed-local" builtinId="0" xfId="1"/></cellStyles></styleSheet>'
class NormalEvidenceTests(unittest.TestCase):
    def test_nonzero_builtin_chain_is_not_cell_xf_zero(self):
        r = normal_snapshot(styles())
        self.assertEqual((r['styleXfIndex'], r['fontIndex'], r['face']), (1, 1, 'Arial'))
        self.assertEqual(r['cellXf0Stored']['fontId'], '0')
    def test_name_and_index_changes_are_not_font_semantic_changes(self):
        a = normal_snapshot(styles()); b = dict(a, styleName='Normal', styleXfIndex=0, fontIndex=0)
        self.assertTrue(same_normal_font(a, b))
    def test_normalization_is_not_excused_by_coincident_mdw(self):
        a = normal_snapshot(styles()); b = dict(a, face='Calibri')
        self.assertEqual(normal_oracle_status(a, b, b), 'office_import_normalized_font_not_dpi_only')
    def test_cell_font_override_does_not_replace_normal_font(self):
        a = normal_snapshot(styles())
        b = normal_snapshot(styles().replace('<xf fontId="0" xfId="0"/>', '<xf fontId="1" xfId="0" applyFont="1"/>'))
        self.assertTrue(same_normal_font(a, b))
if __name__ == '__main__':
    unittest.main()
