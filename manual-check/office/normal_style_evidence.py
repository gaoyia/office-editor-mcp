"""Evidence-only Normal-style resolver. Never substitute cell-XF0 for Normal.

ISO 29500-1 18.8.7 (also confirmed by Microsoft Open Specifications Support):
builtinId=0 -> cellStyle.xfId -> cellStyleXfs[xfId].fontId -> fonts[fontId].
Identifier/name renumbering is not a font-semantics change.
"""
import xml.etree.ElementTree as ET
N = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

def normal_snapshot(data):
    root = ET.fromstring(data)
    styles = list(root.find('m:cellStyles', N))
    normal = next((s for s in styles if s.get('builtinId') == '0'), None)
    if normal is None:
        normal = next(s for s in styles if s.get('name', '').lower() == 'normal')
    index = int(normal.get('xfId', '0'))
    xf = list(root.find('m:cellStyleXfs', N))[index]
    font_index = int(xf.get('fontId', '0'))
    fonts = list(root.find('m:fonts', N))
    font = fonts[font_index]
    def value(tag, default):
        el = font.find('m:' + tag, N)
        return default if el is None else el.get('val', default)
    return dict(styleName=normal.get('name'), builtinId=normal.get('builtinId'),
                styleXfIndex=index, fontIndex=font_index,
                face=value('name', 'Calibri'), pt=float(value('sz', '11')),
                bold=font.find('m:b', N) is not None and value('b', '1') not in ('0', 'false', 'off'),
                italic=font.find('m:i', N) is not None and value('i', '1') not in ('0', 'false', 'off'),
                styleApplyFont=xf.get('applyFont'),
                cellXf0Stored=dict(list(root.find('m:cellXfs', N))[0].attrib))

def same_normal_font(a, b):
    def face(v):
        return 'dengxian' if v.lower() in ('dengxian', '等线') else v.lower()
    return (face(a['face']), a['pt'], a['bold'], a['italic']) == (
        face(b['face']), b['pt'], b['bold'], b['italic'])

def normal_oracle_status(source, office, canonical):
    """Observed face/size only; never pretend unqueried COM bold/italic exist."""
    def identity(v):
        face = v['face'].lower()
        return ('dengxian' if face in ('dengxian', '等线') else face, v['pt'])
    if identity(source) == identity(office):
        return 'face_size_preserved_not_full_font_certificate'
    if identity(office) == identity(canonical):
        return 'office_import_normalized_font_not_dpi_only'
    return 'unresolved_font_evidence'
