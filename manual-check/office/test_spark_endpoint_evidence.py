import unittest
from decimal import Decimal
from PIL import Image
from spark_endpoint_evidence import ink_bounds, ink_profile, edge_crop_preserves_ink, line_weight_pixels, same_native_pixels, kernel_max_coverage

class SparkEndpointEvidenceTests(unittest.TestCase):
    def test_same_bbox_is_not_same_native_pixels(self):
        a=Image.new('RGB',(4,3),'white');b=a.copy();a.putpixel((1,1),(100,100,100));b.putpixel((1,1),(120,120,120))
        self.assertEqual(ink_bounds(a),ink_bounds(b));self.assertFalse(same_native_pixels(a,b))
    def test_profiles_keep_support_not_invented_centerline(self):
        a=Image.new('RGB',(4,3),'white');a.putpixel((1,1),(254,254,254));a.putpixel((2,1),(0,0,0))
        self.assertEqual(ink_profile(a),{'columns':[0,1,1,0],'rows':[0,2,0]});self.assertEqual(ink_bounds(a),(1,1,3,2))
    def test_origin_zero_white_edge_crop_preserves_ink(self):
        a=Image.new('RGB',(4,3),'white');a.putpixel((1,1),(55,96,146));self.assertTrue(edge_crop_preserves_ink(a,a.crop((0,0,3,2))))
    def test_nonwhite_removed_edge_or_shift_is_not_certified(self):
        a=Image.new('RGB',(4,3),'white');a.putpixel((3,1),(55,96,146));self.assertFalse(edge_crop_preserves_ink(a,a.crop((0,0,3,2))))
        a.putpixel((1,1),(55,96,146));self.assertFalse(edge_crop_preserves_ink(a,a.crop((1,0,4,2))))
    def test_physical_weight_is_not_bbox_height(self):
        self.assertEqual(line_weight_pixels('.75',96),Decimal(1));self.assertEqual(line_weight_pixels('.75',106),Decimal(106)/96)
        self.assertEqual(line_weight_pixels(3,106),Decimal(53)/12)
    def test_audited_kernel_cannot_reach_native_opaque_center_even_at_106(self):
        self.assertEqual(kernel_max_coverage(line_weight_pixels('.75',96),'.25'),Decimal('.75'))
        bound=kernel_max_coverage(line_weight_pixels('.75',106),'.25')
        self.assertLess(bound,Decimal('.803'))
        self.assertLess(bound,Decimal(1))

    def test_invalid_weights_and_dpi_are_rejected(self):
        for points,dpi in [(0,96),(-1,96),(1,0),('NaN',96),(1,'Infinity')]:
            with self.assertRaises(ValueError):line_weight_pixels(points,dpi)

if __name__=='__main__':unittest.main()
