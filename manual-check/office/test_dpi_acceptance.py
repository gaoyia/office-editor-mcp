import unittest
from dpi_acceptance import classify, PASS, DIFFERENCE, NI, PENDING

class DpiAcceptanceTests(unittest.TestCase):
    def proof(self, **overrides):
        p = dict(scope="shared_geometry_only", source_verified=True, renderer_verified=True, semantic_verified=True,
                 font_verified=True, dpi_verified=True, raw_strict='sizeDiffer')
        p.update(overrides)
        return p
    def test_size_differ_alone_never_passes(self):
        self.assertEqual(classify(raw_strict='sizeDiffer'), PENDING)
    def test_proven_dpi_only_named_scope_passes(self):
        self.assertEqual(classify(**self.proof()), PASS)
        self.assertEqual(classify(**self.proof(scope=None)), PENDING)
    def test_same_width_cannot_hide_different_normal_font(self):
        self.assertEqual(classify(**self.proof(font_verified=False)), PENDING)
    def test_source_sha_mismatch_cannot_pass(self):
        self.assertEqual(classify(**self.proof(source_verified=False)), PENDING)
    def test_stale_renderer_cannot_pass(self):
        self.assertEqual(classify(**self.proof(renderer_verified=False)), PENDING)
    def test_real_position_difference_is_not_dpi(self):
        self.assertEqual(classify(**self.proof(implementation=['label_side'])), DIFFERENCE)
    def test_unknown_curve_or_endpoint_aa_is_not_waived(self):
        for item in ['curve_aa', 'spark_endpoint', 'manual_position']:
            self.assertEqual(classify(**self.proof(unresolved=[item])), PENDING)
    def test_ni_is_honest_and_separate(self):
        self.assertEqual(classify(**self.proof(ni=True)), NI)
    def test_subscope_pass_does_not_promote_whole_image(self):
        self.assertEqual(classify(**self.proof()), PASS)
        self.assertEqual(classify(**self.proof(unresolved=['whole_image_unchecked'])), PENDING)
    def test_raw_pass_is_not_semantic_evidence(self):
        self.assertEqual(classify(raw_strict='PASS'), PENDING)

if __name__ == '__main__':
    unittest.main()
