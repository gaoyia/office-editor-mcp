import math
import unittest
from chart_boundary_evidence import device_grid_px, device_span_px, native_width_matches, png_phys_compatible, rgba_equal
class ChartBoundaryEvidenceTests(unittest.TestCase):
    def test_double_column_grid_does_not_gain_a_pixel_from_ceil(self):
        self.assertEqual(device_grid_px(54.339622641509436, 106), 80)
        self.assertEqual(device_span_px(0, 217.35849056603774, 106), 320)
        self.assertEqual(math.ceil(217.35850524902344*106/72), 321)
    def test_single_shape_readback_is_not_a_double_column_grid(self):
        with self.assertRaises(ValueError):
            device_grid_px(217.35850524902344, 106)
    def test_fractional_and_real_one_pixel_mismatch_are_not_waived(self):
        with self.assertRaises(ValueError):
            device_grid_px(80.25*72/106, 106)
        self.assertFalse(native_width_matches(321, [54.339622641509436]*4, 106))
    def test_hidden_columns_and_device_endpoint_span(self):
        self.assertEqual(device_grid_px(0, 106), 0)
        cols=[55.0188679245283,55.0188679245283,0,55.0188679245283]
        self.assertTrue(native_width_matches(243, cols, 106))
        self.assertEqual(device_span_px(0,165.0566037735849,106),243)
    def test_phys_rounding_does_not_replace_public_dpi(self):
        self.assertTrue(png_phys_compatible(4173,106))
        self.assertFalse(png_phys_compatible(4173,105))
    def test_rgb_changes_cannot_hide_behind_zero_difference_alpha(self):
        from PIL import Image
        a=Image.new('RGBA',(1,1),(0,0,0,255));b=Image.new('RGBA',(1,1),(1,0,0,255))
        self.assertFalse(rgba_equal(a,b))
        self.assertTrue(rgba_equal(a,a.copy()))

if __name__ == '__main__':
    unittest.main()
