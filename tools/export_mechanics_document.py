"""Export the mechanics Markdown to an editable Word reference.

Requires Pandoc and python-docx. Run from any directory; optional --pandoc
selects a specific executable. Rendering and visual review remain separate.
"""
import argparse
import re
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import quote

from docx import Document
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches, Pt, RGBColor

ROOT = Path(__file__).resolve().parents[1]


def element(tag, **attrs):
    node = OxmlElement(tag)
    for key, value in attrs.items():
        node.set(qn('w:' + key), str(value))
    return node


def export(pandoc, output):
    source = ROOT / 'docs/RACE-MECHANICS.md'
    text = source.read_text(encoding='utf-8')

    def public_link(match):
        target = match.group(1)
        if '://' in target or target.startswith('#'):
            return match.group(0)
        path, separator, anchor = target.partition('#')
        relative = (source.parent / path).resolve().relative_to(ROOT).as_posix()
        url = 'https://github.com/daftuyda/GlobalSim/blob/main/' + quote(relative)
        return '](' + url + (separator + anchor if separator else '') + ')'

    text = re.sub(r'\]\(([^)]+)\)', public_link, text)
    with tempfile.TemporaryDirectory(prefix='globalsim-mechanics-') as scratch:
        markdown = Path(scratch) / 'reference.md'
        intermediate = Path(scratch) / 'reference.docx'
        markdown.write_text(text, encoding='utf-8')
        subprocess.run([pandoc, str(markdown), '--from=gfm+tex_math_dollars',
                        '--to=docx', '--standalone', '--output', str(intermediate)], check=True)
        doc = Document(intermediate)

    # Older Pandoc templates capitalize builtin names that python-docx aliases.
    for style in doc.styles:
        if re.fullmatch(r'Heading[1-9]', style.style_id):
            style.name = 'heading ' + style.style_id[-1]

    section = doc.sections[0]
    section.page_width, section.page_height = Inches(8.5), Inches(11)
    section.top_margin = section.bottom_margin = Inches(.75)
    section.left_margin = section.right_margin = Inches(.8)
    section.header_distance = section.footer_distance = Inches(.3)
    body = doc.styles['Normal']
    body.font.name, body.font.size = 'Arial', Pt(11)
    body.paragraph_format.line_spacing = 1.08
    body.paragraph_format.space_after = Pt(7)
    for name, size in [('Title', 25), ('Heading 1', 16), ('Heading 2', 12)]:
        style = doc.styles[name]
        style.font.name, style.font.size = 'Arial', Pt(size)
        style.font.color.rgb = RGBColor.from_string('000000')
        style.font.bold, style.font.underline = True, False
        style.paragraph_format.space_before = Pt(15 if name != 'Title' else 0)
        style.paragraph_format.space_after = Pt(7)
        style.paragraph_format.keep_with_next = True
        for node in list(style.element.findall('.//' + qn('w:pBdr'))):
            node.getparent().remove(node)

    chapters = []
    for i, paragraph in enumerate(doc.paragraphs):
        if i == 0:
            paragraph.style = doc.styles['Title']
        elif paragraph.style.name == 'Heading 2':
            paragraph.style = doc.styles['Heading 1']
            chapters.append(paragraph)
        elif paragraph.style.name == 'Heading 3':
            paragraph.style = doc.styles['Heading 2']
        if paragraph.style.name in ('Source Code', 'Verbatim Char'):
            paragraph.paragraph_format.space_after = Pt(2)
            paragraph.paragraph_format.keep_together = True
            for run in paragraph.runs:
                run.font.name, run.font.size = 'Consolas', Pt(10)
        paragraph.paragraph_format.widow_control = True

    # A compact static contents list keeps the document usable without field refresh.
    first = chapters[0]
    heading = first.insert_paragraph_before('Contents')
    heading.style = doc.styles['Heading 1']
    for chapter in chapters:
        paragraph = first.insert_paragraph_before(chapter.text)
        paragraph.paragraph_format.space_after = Pt(2)
        paragraph.paragraph_format.line_spacing = 1
        for run in paragraph.runs:
            run.font.size = Pt(10)
    first.paragraph_format.page_break_before = True

    for table in doc.tables:
        table.autofit = False
        count = len(table.columns)
        if count == 0:
            count = sum(cell.grid_span for cell in table._tbl.tr_lst[0].tc_lst)
            for _ in range(count):
                table._tbl.tblGrid.append(element('w:gridCol', w=1200))
        width = 6.9
        first_width = {2: 2.25, 3: 2.3, 4: 2.25, 5: 2.0, 9: 2.0}.get(count, width / count)
        widths = [first_width] + [(width - first_width) / (count - 1)] * (count - 1)
        for column, value in zip(table.columns, widths):
            column.width = Inches(value)
        props = table._tbl.tblPr
        for old in list(props.findall(qn('w:tblBorders'))):
            props.remove(old)
        borders = element('w:tblBorders')
        for side in ('top', 'left', 'bottom', 'right', 'insideH', 'insideV'):
            borders.append(element('w:' + side, val='single', sz='4', color='D9D9D9'))
        props.append(borders)
        for row_index, row in enumerate(table.rows):
            row._tr.get_or_add_trPr().append(element('w:cantSplit'))
            if row_index == 0:
                row._tr.get_or_add_trPr().append(element('w:tblHeader'))
            for index, cell in enumerate(row.cells):
                cell.width = Inches(widths[index])
                cell.vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.CENTER
                cp = cell._tc.get_or_add_tcPr()
                for old in list(cp.findall(qn('w:tcBorders'))):
                    cp.remove(old)
                cell_borders = element('w:tcBorders')
                for side in ('top', 'left', 'bottom', 'right'):
                    cell_borders.append(element('w:' + side, val='single', sz='4', color='D9D9D9'))
                cp.append(cell_borders)
                margins = element('w:tcMar')
                for side in ('top', 'bottom', 'left', 'right'):
                    margins.append(element('w:' + side, w=80, type='dxa'))
                cp.append(margins)
                cp.append(element('w:shd', fill='DCE6F1' if row_index == 0 else 'FFFFFF'))
                for paragraph in cell.paragraphs:
                    paragraph.paragraph_format.space_after = Pt(3)
                    paragraph.paragraph_format.space_before = Pt(3)
                    paragraph.paragraph_format.line_spacing = 1.0
                    paragraph.paragraph_format.keep_with_next = row_index == 0
                    paragraph.alignment = WD_ALIGN_PARAGRAPH.LEFT if index == 0 or count == 2 else WD_ALIGN_PARAGRAPH.CENTER
                    for run in paragraph.runs:
                        run.font.name, run.font.size = 'Arial', Pt(10)
                        run.font.bold = row_index == 0

    header = section.header.paragraphs[0]
    header.text = 'GlobalSim Race Mechanics  |  Version 77'
    for run in header.runs:
        run.font.name, run.font.size = 'Arial', Pt(9)
        run.font.color.rgb = RGBColor.from_string('000000')
    footer = section.footer.paragraphs[0]
    footer.alignment = WD_ALIGN_PARAGRAPH.CENTER
    footer.add_run('GlobalSim  |  ')
    field = element('w:fldSimple', instr='PAGE')
    footer._p.append(field)
    for run in footer.runs:
        run.font.name, run.font.size = 'Arial', Pt(9)
    doc.core_properties.title = 'GlobalSim Race Mechanics'
    doc.core_properties.subject = 'Version 77 implementation reference'
    doc.core_properties.author = 'GlobalSim contributors'
    doc.core_properties.keywords = 'GlobalSim, race mechanics, v77'
    output.parent.mkdir(parents=True, exist_ok=True)
    doc.save(output)
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pandoc', default='pandoc')
    parser.add_argument('--output', type=Path, default=ROOT / 'docs/GlobalSim-Race-Mechanics-v77.docx')
    args = parser.parse_args()
    export(args.pandoc, args.output)
