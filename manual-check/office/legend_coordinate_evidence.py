"""Legend coordinate evidence: chart-local COM values are not PNG ink boxes."""
import xml.etree.ElementTree as ET


def worksheet_area_local(area, chart_object):
    return {'left':area['left']-chart_object['left'], 'top':area['top']-chart_object['top'], 'width':area['width'], 'height':area['height']}


def translation_certifies_chart_local(before, after, same_native_png):
    a, b = before['chartObjectWorksheetRect'], after['chartObjectWorksheetRect']
    return (a['left'] != b['left'] and a['top'] != b['top']
            and a['width'] == b['width'] and a['height'] == b['height']
            and before['legendRect'] == after['legendRect']
            and before['keys'] == after['keys'] and same_native_png)


def nominal_point_rect(rect, dpi):
    """Scale only. Never imply that the unmeasured chart-internal origin is zero."""
    if dpi <= 0:
        raise ValueError('positive DPI required')
    return {'left':rect['left']*dpi/72, 'top':rect['top']*dpi/72,
            'width':rect['width']*dpi/72, 'height':rect['height']*dpi/72,
            'pngOriginCalibrated':False}


def legend_without_overlay(xml):
    root = ET.fromstring(xml)
    for child in list(root):
        if child.tag.rsplit('}',1)[-1] == 'overlay':
            root.remove(child)
    return ET.tostring(root, encoding='unicode')
