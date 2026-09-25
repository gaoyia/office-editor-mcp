# Annex J.  (informative) Accessibility Best Practices

<!-- source-pages 4513-4545 -->

<!-- page 4513 -->

Annex J

4503
Annex J.
(informative)
Accessibility Best Practices
This annex is informative.
This annex is a collection of best practices for developers and document/template authors
implementing ECMA-376, and for customers using such implementations, with the goal of maximizing
the accessibility of Office Open XML documents.
Office Open XML provides a rich infrastructure for creating content that meets the needs of people with
disabilities.  Authors and developers are encouraged to follow these guidelines in order to enable users
with disabilities to consume content or to extract the full meaning of Office Open XML documents.
J.1
The Value of Creating an Accessible Office Open XML
Implementation
There are numerous benefits associated with creating an accessible Office Open XML
implementation.  Office Open XML provides a rich infrastructure for creating content that meets the
needs by people with disabilities, and enables them to read, create, and edit documents with full access
to all of the semantics captured by the documents’ creators.
In addition to the moral and ethical reasons for creating accessible Office Open XML documents, it
should be noted that failure to provide support for users with disabilities can have serious legal
implications.  Examples of these regulations include

Section 508 of the U.S. Rehabilitation Act (508) (http://www.section508.gov)

Section 255 of the U.S. Telecommunications Act of 1996
(http://www.fcc.gov/cgb/dro/telecom_language.html)

International Organization for Standardization (ISO/TC 171)

Authoring Tool Accessibility Guidelines (ATAG) (http://www.w3.org/TR/ATAG10/)

Web Content Accessibility Guidelines (WCAG) http://www.w3.org/TR/WCAG20/

User Agent Accessibility Guidelines (UAAG) http://www.w3.org/TR/UAAG10/

Canadian Common Look and Feel (CLF) Guide 1.1 http://www.cio-dpi.gc.ca/clf-upe/1/1_e.asp
While there are laws (such as those above) designed to ensure that businesses do everything possible to
maximize accessibility, there are other compelling reasons as well.  Today, businesses are looking for
solutions to empower and retain employees—and accessible technology can help do just that.
Accessible technology helps businesses keep great employees, recruit from a larger pool of candidates,

<!-- page 4514 -->

ECMA-376 Part 1

4504

and enhance team collaboration and communication among all employees—including those with
disabilities.
J.2
Needs by Type of Disability
Disabilities can be thought of as fitting into one of five categories (with some needs spanning multiple
categories).  These include vision, dexterity, hearing, learning, and communication.  Details on the
definition of each category, and the technologies that exist to address the needs today, follow below.
J.2.1
Vision
Vision difficulties and impairments include low vision, color blindness, blindness, and even conditions
such as presbyopia, which is associated with aging populations.  For individuals with vision difficulties,
there are many options to modify the computer display and appearance so it is more legible, or to
receive information through sound or touch. Those who are blind cannot use a computer monitor but
have the option to receive information from their computers through hearing or touch offered through
screen readers and Braille displays, respectively.
People with vision difficulties and impairments might be interested in the following assistive technology:
1. Screen enlargers (or screen magnifiers) work like a magnifying glass. They enlarge a portion of
the screen as the user moves the focus—increasing legibility for some users. Some screen
enlargers allow a user to zoom in and out on a particular area of the screen.
2. Screen readers are software programs that present graphics and text as speech. A screen reader
is used to verbalize, or "speak", everything on the screen including names and descriptions of
control buttons, menus, text, and punctuation.
3. Speech recognition systems, also called voice recognition programs, allow people to give
commands and enter data using their voices rather than a mouse or keyboard.
4. Speech synthesizers (often referred to as text-to-speech [TTS] systems) receive information
going to the screen in the form of letters, numbers, and punctuation marks, and then "speak" it
out loud. The use of speech synthesizers allows blind users to review their input as they type.
5. Refreshable Braille displays provide tactile output of information represented on the computer
screen. The user reads the Braille letters with his or her fingers, and then, after a line is read,
refreshes the display to read the next line.
6. Braille embossers transfer computer generated text into embossed Braille output. Braille
translation programs convert text scanned in, or generated via, standard word processing
programs into Braille, which can be printed on the embosser.
7. Talking word processors are software programs that use speech synthesizers to provide auditory
feedback of what is typed.
8. Large-print word processors allow the user to view everything in large text without added
screen enlargement.

<!-- page 4515 -->

Annex J

4505
J.2.2
Dexterity
Individuals with dexterity difficulties experience pain, discomfort, or complete loss of feeling in their
fingers, hands, wrists, or arms, making it difficult to use a standard keyboard or mouse. Dexterity
difficulties and impairments can be caused by a wide range of common illnesses and accidents such as
carpal tunnel, arthritis, stroke, cerebral palsy, Parkinson's disease, multiple sclerosis, loss of limbs or
digits, spinal cord injuries, and repetitive stress injury.
People with dexterity difficulties and impairments might be interested in the following assistive
technology:
1. Speech recognition systems, also called voice recognition programs, allow people to give
commands and enter data using their voices rather than a mouse or keyboard.
2. On-screen keyboard programs provide an image of a standard or modified keyboard on the
computer screen. The user selects the keys with a mouse, touch screen, trackball, joystick,
switch, or electronic pointing device.
3. Keyboard filters include typing aids, such as word prediction utilities and add-on spelling
checkers. These products reduce the required number of keystrokes. Keyboard filters enable
users to access quickly the letters they need and to avoid inadvertently selecting keys they don't
want.
4. Touch screens are devices placed on the computer monitor (or built into it) that allow direct
selection or activation of the computer by touching the screen.
5. Alternative input devices (including alternative keyboards, electronic pointing devices, sip-and-
puff systems, wands and sticks, joysticks and trackballs) allow individuals to control their
computers through means other than a standard keyboard or pointing device.
J.2.3
Hearing
Hearing difficulties and impairments encompass a wide range of conditions—from slight hearing loss to
deafness. People with hearing difficulties and impairments might be able to hear some sound, but might
not be able to distinguish words.
J.2.4
Learning Difficulties
Learning difficulties and impairments can range from conditions such as dyslexia and attention deficit
disorder to retardation. Processing problems are the most common and have the most impact on a
person's ability to use computer applications. These conditions interfere with the learning process.
Many individuals with learning difficulties and impairments are of high intelligence and if information is
presented to them in a form and at a pace that is appropriate, they are able to be much more
productive. During the learning process, many individuals with learning difficulties benefit from having a
multisensory experience of audio speech paired with a visual representation. Reducing visual and
auditory distractions can also aid the learning process for many people.  People with learning difficulties
and impairments might be interested in the following:

<!-- page 4516 -->

ECMA-376 Part 1

4506

1. Word prediction programs allow the user to select a desired word from an on-screen list located
in the prediction window. This list, generated by the computer, predicts words from the first one
or two letters typed by the user. The word can then be selected from the list and inserted into
the text by typing a number, clicking the mouse or scanning with a switch. These programs help
users increase written productivity and accuracy, and increase vocabulary skills through word
prompting.
2. Reading comprehension programs focus on establishing or improving reading skills through
ready-made activities, stories, exercises, or games. These programs can help users practice
letter sound recognition and can increase the understanding of words by adding graphics,
sound, and possibly animation.
3. Reading tools and learning disability programs include software designed to make text-based
materials more accessible for people who struggle with reading. Options can include scanning,
reformatting, navigating, or speaking text out loud. These programs help people who have
difficulty seeing or manipulating conventional print materials; people who are developing new
literacy skills or who are learning English as a foreign language; and people who comprehend
better when they hear and see text highlighted simultaneously.
4. Speech synthesizers (often referred to as text-to-speech [TTS] systems) receive information
going to the screen in the form of letters, numbers, and punctuation marks, and then "speak" it
out loud. Individuals who have lost the ability to communicate orally can use a speech
synthesizer to communicate by typing information and letting the speech synthesizer speak it
out loud.
5. Speech recognition systems, also called voice recognition programs, allow people to give
commands and enter data using their voices rather than a mouse or keyboard.
J.2.5
Communication
Language and communication difficulties and impairments include conditions such as aphasia (loss or
impairment of the power to use or comprehend words, often as a result of brain damage), delayed
speech (a symptom of cognitive impairment), and other conditions resulting in difficulties remembering,
solving problems, or perceiving sensory information. For people who have these difficulties and
impairments, complex or inconsistent visual displays or word choices can make using computers more
difficult.  People who have language or communication difficulties and impairments might be interested
in the following assistive technology:
1. Keyboard filters include typing aids such as word prediction utilities and add-on spelling
checkers. These products reduce the required number of keystrokes. Certain keyboard filters
enable users to quickly access the letters they need and to avoid inadvertently selecting keys
they don't want.
2. Speech recognition systems, also called voice recognition programs, allow people to give
commands and enter data using their voices rather than a mouse or keyboard.
3. Screen review utilities make on-screen information available as synthesized speech and pairs the
speech with a visual representation of a word, for example, highlighting a word as it is spoken.
Screen review utilities convert the text that appears on screen into a computer voice. This helps

<!-- page 4517 -->

Annex J

4507
some people with language difficulties and impairments by giving them information visually and
aurally at the same time.
4. Touch screens are devices placed on the computer monitor (or built into it) that allow direct
selection or activation of the computer by touching the screen.
5. Speech synthesizers (often referred to as text-to-speech [TTS] systems) receive information
going to the screen in the form of letters, numbers, and punctuation marks, and then "speak" it
out loud.
J.3
Best Practices for Developers
Developers are encouraged to deliver a core set of functionality that can best enable customers to both
produce and consume highly accessible documents.
J.3.1
Development Principles for Developing Highly Accessible
Implementation
Here is the set of design principles that every developer should adhere to when implementing for
accessibility:
1. Product UI controls and content should be accessible. This includes electronic forms, UI controls,
self contained software products, scripts, applets, plug-ins, and all other programmatic elements
that generate or interpret page content.
2. Product UI controls and content should be compatible with assistive technologies. A user can
thus employ assistive technology, such as screen readers, magnifiers, voice recognition, and
alternative input devices to help them use the product. However, if the product is a self-
contained product such as a kiosk or cell phone, for example, it should not require that such a
user add assistive technology in order to use the product.
3. If a product is an authoring tool, it should enable an author to edit the structure of a document,
as well as all of the properties of each element and object, in an accessible fashion.
4. A product should address the needs of people with vision difficulties, hearing difficulties,
cognitive difficulties, or motor difficulties. Specifically, the implementation should provide at
least one mode of operation and information retrieval that does not require that a user be able
to see, hear, speak, move with fine motor control, do two things at once, reach, or exert
physical strength. Except in the case of a self-contained product, the product may accomplish
this by facilitating the use of assistive technology developed by third-party vendors.
J.3.2
Navigation
There are numerous things a developer should consider with respect to navigation in order to create an
accessible Office Open XML implementation.  At a high level these include the following:
1. It should be possible to do everything using only the keyboard.  There should be no functions
that are dependent on the input of a mouse or other similar device.

<!-- page 4518 -->

ECMA-376 Part 1

4508

2. It should also be efficient to do everything via the keyboard.  For example, it is unreasonable to
require keyboard users to press TAB to navigate through hundreds of controls, just to get to the
control that interests them.
3. It should be possible to do everything using only one hand.  Shortcut key sequences should be
reasonable to perform with one hand.
4. Keyboard navigation should be consistent and reasonable.  TAB should navigate from one
control to another.  ESC should cancel the current keyboard state.  The most commonly used
controls should be visited first in the TAB order.  If there are numerous controls laid out in two
dimensions, the arrow keys should also work for two-dimensional keyboard navigation.
J.3.3
Keyboard Focus
Users must know which object has the keyboard focus at anytime.  Avoid confusion by hiding all visual
focus indicators and dimming selections that are located in inactive windows (or panes).  To highlight
the keyboard focus, use colors, fonts, graphics such as rectangles, or magnification. Highlight the
keyboard focus audibly by changing the volume, the pitch, or the tonal quality.  In addition an accessible
Office Open XML implementation should ensure that:
1. An object always has the keyboard focus, and the keyboard focus is always visible and obvious.
When the focus moves to another element, the new position is obvious.
2. If a UI control currently has the focus, it is visually highlighted.
3. All selections are visually highlighted.
4. The keyboard focus is displayed independently of a selection, allowing support for multiple and
non-contiguous selections.
5. The implementation can show a selection that exists in an inactive window or pane; this can be
done, for example, by dimming the selection.
6. Only one keyboard focus indicator shows at any one time.
7. Users can place the focus anywhere within the textual content or on any UI control with which
they can interact.
J.3.4
Existing Navigation Guidelines
An accessible Office Open XML implementation should follow the keystroke guidelines provided by the
OS on which the application is built.  Guidelines exist for most major operating systems including
Windows, Macintosh, and Linux.
J.3.5
Themes and Contrast Support
Users often have unique demands of their computing environment (e.g., high contrast by default) and
the settings associated with these demands should be followed by any accessible Office Open XML
implementation.  System-wide settings should never be disabled or disregarded by a product if they are
intended to increase accessibility.  Many users require specific high-contrast combinations, such as
white text on a black background. Drawing these reversed, as black text on a white background, causes
the background to bleed over the foreground and can make reading difficult, even painful, for some
users.  Text is most legible for everyone when drawn on a plain background of a contrasting color. Text

<!-- page 4519 -->

Annex J

4509
drawn over a variegated background, such as a wash of colors or a bitmap, can be difficult to read for
the person without disabilities, and can be unreadable for users who have turned on a high contrast-like
mode.
J.3.6
Assistive Technologies (AT)
Any accessible Office Open XML application should provide at least one mode that allows for full use of
the product without the use of vision or by interacting directly with the users’ assistive technologies.
Failure to provide at least one of these options results in certain users having no means by which to use
an application.
J.3.6.1
Programmatic Access
A programmatic accessibility technology is fundamentally a method of providing information to the AT
through some publically defined API (or set of API’s).  Assistive Technologies should be able to identify
and manipulate the elements of an application’s user interface.  The standard mechanisms of an
application should ensure that AT has access to any content presented textually or through other visual
representations on the screen.  An accessible Office Open XML implantation should do this by:
1. Providing programmatic access through one of the many accessibility frameworks available
through operating systems.  An example of this would be UI Automation.   UI Automation
provides programmatic access to most user interface (UI) elements on the desktop, enabling
assistive technology products such as screen readers to provide information about the UI to end
users and to manipulate the UI by means other than standard input. UI Automation also allows
automated test scripts to interact with the UI. UI Automation client applications can be written
with the assurance that they will work on multiple frameworks. The UI Automation core masks
any differences in the frameworks that underlie various pieces of UI. [Example: The Content
property of a WPF button, the Caption property of a Win32 button, and the ALT property of an
HTML image are all mapped to a single property, Name, in the UI Automation view. end
example]
2. Other methods such as the use of a document object model (DOM).  A DOM can be defined as
the document-level information source.  [Example: The Document Object Model (DOM) as
implemented in MSXML provides a programmatic representation of XML documents, fragments,
nodes, or node-sets. It also provides an application programming interface for working with XML
data. As an XML representation, it conforms to the W3C DOM specification. As a set of APIs,
XML DOM objects are COM objects that implement interfaces and can be used in XML
applications written in programming languages such as C/C++, Visual Basic, VBScript, and JScript.
end example]
J.3.6.2
Additional Programmatic Access
The following guidelines should be followed to ensure an accessible Office Open XML specification:
1. The system caret (the blinking vertical bar that users see when editing text) exposes the location
of the keyboard focus for assistive technology. The system caret can be placed anywhere on the

<!-- page 4520 -->

ECMA-376 Part 1

4510

screen and made any shape or size. It can also be made invisible and moved to indicate the
keyboard focus location for the benefit of other applications without disturbing what the user
sees on the screen.  The system caret should be sized and positioned to cover the screen
element's bounding rectangle, so that screen magnification tools can zoom in on any part of the
complete object. This helps the user look for an object's label or judge its size, and it enables
utilities to highlight the object that has the keyboard focus.
2. The application should provide programmatic access to all textual content and all text
attributes.
3. The application associates a unique, meaningful title (and a description, if needed) with frames,
objects, windows, and Web pages to facilitate identification and navigation.
4. The description associated with a frame, object, window, or Web page gives the purpose of the
item and explains how that item relates to other items. (A description is necessary only if the
purpose and relationship is not clear from the title alone.)
5. Vector drawings and elements have title and desc tags. The alternative text is reused as
graphics.
6. The application should provide a mechanism by which the user can view headers (row and
column) for each cell in a table.
J.3.7
Application Help
The help system of an Office Open XML system should also be accessible.  In addition, developers should
do everything possible to make access to a help system as easy possible.  Advanced methods for
achieving this include providing direct access to help topics via any features in the UI.   Searchable help
systems tend to be
J.4
Best Practices for Document and Template Authors
When generating documents and templates based on Office Open XML, there are a set of best practices
that should be followed to best make use of the structure implicit in Office Open XML.  The following
paragraphs describe these practices.  These practices are not necessarily the only way to enhance each
scenario.
J.4.1
Understanding the Type of Document
The first step in being able to consume a document is to understand the type of document one will be
consuming.  Office Open XML supports three primary types of documents: documents, presentations
and spreadsheets.  The type of document is determined programmatically by looking at the content type
of the start part.
The first step is getting the content type is to identify the target part of the ‘officedocument’
relationship.  To do this, one needs to start in the /_rels/.rels file and identify the relationship of type
http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument:

<!-- page 4521 -->

Annex J

4511
<Relationships
xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId3"

Type="http://purl.oclc.org/ooxml/officeDocument/relationships/extendedPrope
rties"
    Target="docProps/app.xml"/>
  <Relationship Id="rId2"

Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata
/core-properties"
    Target="docProps/core.xml"/>
  <Relationship Id="rId1"

Type="http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocumen
t"
    Target="word/document.xml"/>
</Relationships>
Then, given the target of that relationship, in this case “word/document.xml,” the [Content_Types].xml
file is used to look up the content type for this target.  Given the following [Content_Types].xml file,
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-
types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-
package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml"
    ContentType="application/vnd.openxmlformats-
officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml"
    ContentType="application/vnd.openxmlformats-
officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/docProps/app.xml"
    ContentType="application/vnd.openxmlformats-officedocument.extended-
properties+xml"/>
  <Override PartName="/word/settings.xml"
    ContentType="application/vnd.openxmlformats-
officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/theme/theme1.xml"
    ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/word/fontTable.xml"
    ContentType="application/vnd.openxmlformats-
officedocument.wordprocessingml.fontTable+xml"/>

<!-- page 4522 -->

ECMA-376 Part 1

4512

  <Override PartName="/word/webSettings.xml"
    ContentType="application/vnd.openxmlformats-
officedocument.wordprocessingml.webSettings+xml"/>
  <Override PartName="/docProps/core.xml"
    ContentType="application/vnd.openxmlformats-package.core-
properties+xml"/>
</Types>
The content type is application/vnd.openxmlformats-
officedocument.wordprocessingml.document.main+xml, which tells us that the file is a
document file.
J.4.2
Understanding the Function of a Document
With the understanding that it is a futile exercise to attempt to categorize every type of document
function, Office Open XML leaves it to developers, and document authors, to generate functionally-
specific templates for use by themselves and others.  It is in this spirit that the function of most
documents can be hinted at by looking at the template used.
Consider a presentation is created using a template for 2008 External Pitch Books.  By examining the
application properties part ( \docProps\appdata.xml ), one can find the template element that defines
the template used to create the presentation.
For example, the previously mentioned presentation’s appdata would look like this:
<Properties xmlns =
    "http://purl.oclc.org/ooxml/officeDocument/extendedProperties"
  xmlns:vt =
    "http://purl.oclc.org/ooxml/officeDocument/docPropsVTypes">
  <Template>2008 External Pitch Book</Template>
  <TotalTime>0</TotalTime>
  <Words>4</Words>
  …
</Properties>
By leveraging the template property in this way, developers and document authors have the flexibility to
define the taxonomy of document functionalities that suits their particular needs.
It is worth pointing out that there are other mechanisms that can be used to evaluate the function of a
document.  An additional method would be to add a custom document property.  The XML below adds a
new property called “Document Function” in the custom.xml part that specifies the function of the
current document.

<!-- page 4523 -->

Annex J

4513
<Properties
xmlns="http://purl.oclc.org/ooxml/officeDocument/customProperties"
xmlns:vt="http://purl.oclc.org/ooxml/officeDocument/docPropsVTypes">
  <property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2"
    name="Document Function">
    <vt:lpwstr>Quarterly Sales Report</vt:lpwstr>
  </property>
</Properties>
J.4.3
Understanding the Length of a Document
Similarly, the length of a document is easy to ascertain using simple document properties. For a simple
presentation, the Slides element in appdata.xml describes the number of slides in a presentation.
<Properties
xmlns="http://purl.oclc.org/ooxml/officeDocument/extendedProperties"
xmlns:vt="http://purl.oclc.org/ooxml/officeDocument/docPropsVTypes">
  <TotalTime>0</TotalTime>
  <Words>0</Words>
  <Application>Microsoft Office PowerPoint</Application>
  <PresentationFormat>On-screen Show (4:3)</PresentationFormat>
  <Paragraphs>0</Paragraphs>
  <Slides>21</Slides>
  <Notes>0</Notes>
  …
</Properties>
Similar metrics for lengths of documents can be found for both documents and
spreadsheets.  Documents, for example, can express their lengths in a variety of ways, including, but not
limited to, the number of pages, the number of words and even the number of characters.
<Properties
xmlns="http://purl.oclc.org/ooxml/officeDocument/extendedProperties"
xmlns:vt="http://purl.oclc.org/ooxml/officeDocument/docPropsVTypes">
  <Template>Disposition of Comment Template</Template>
  <TotalTime>0</TotalTime>
  <Pages>5</Pages>
  <Words>413</Words>
  <Characters>2356</Characters>
  …
</Properties>
Higher-order constructs can also be used to denote length like the number of sections in documents, the
number of slides in a custom show in presentations and the number of worksheets in spreadsheets.

<!-- page 4524 -->

ECMA-376 Part 1

4514

Additionally, implementers of Office Open XML documents are allowed to provide their own set of
length metrics by way of custom properties.
J.4.4
Understanding the Important Parts in a Document
Again, given the variety of documents that are created, it would be futile to create a structure that all
documents need to fit into.  Hence, it is recommended that the most effective way for developers to
identify important parts of a document is based on common primitives in ECMA-376.
A prime example of this would be identifying the titles of each slide.  Each slide can have a title
placeholder that, generally, represents the title for that particular slide.  The title itself is easily accessed
by looking for the title placeholder in slide.xml.
By looking into the current slide’s shape tree, one can evaluate each shape’s non-visual properties to see
which shape is the title placeholder:
<p:spTree>
    <p:nvGrpSpPr>…</p:nvGrpSpPr>
    <p:grpSpPr>…</p:grpSpPr>
    <p:sp>
        <p:nvSpPr>
            <p:cNvPr id="2" name="Title 1"/>
            <p:cNvSpPr>
                <a:spLocks noGrp="1"/>
            </p:cNvSpPr>
            <p:nvPr>
                <p:ph type="ctrTitle"/>
            </p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
            <a:bodyPr/>
            <a:lstStyle/>
            <a:p>
                <a:r>
                    <a:rPr lang="en-US" dirty="0" smtClean="0"/>
                    <a:t>Strategic Initiatives for Growth</a:t>
                </a:r>
                <a:endParaRPr lang="en-US" dirty="0"/>
            </a:p>
        </p:txBody>
    </p:sp>
    …
<p:/spTree>

<!-- page 4525 -->

Annex J

4515
A similar approach can be used to identify which shapes on a slide are placeholders versus floating
objects, or even which shapes on a slide are shapes which contain text and those which are shapes that
don’t contain text.
J.4.5
Understanding the Organization of a Table via Table Headings
Tables are an often-seen element of many documents.  As tables become more complex, with multi-
level headers, they can become unreadable to certain customers if multi-level headings are not
supported in the document.  Fortunately, Office Open XML does support multi-level headings.
Consider the following table:
(H1) Married filing jointly and you have -
(H2) No Children
(H3) One Child
(H4) Two Children
(H5) Your Credit Is-
$100
$200
$400

The mark-up for such a table would be as follows:
…
<w:tr>
    <w:tc w:id= “H1”>
          <w:tcPr>
                <w:colSpan w:val="3"/>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>Married filing jointly and you have -</w:t>
                </w:r>
          </w:p>
    </w:tc>
</w:tr>
<w:tr>
    <w:tc w:id= “H2”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                </w:headers>
          </w:tcPr>

<!-- page 4526 -->

ECMA-376 Part 1

4516

          <w:p>
                <w:r>
                      <w:t>No Children</w:t>
                </w:r>
          </w:p>
    </w:tc>
    <w:tc w:id= “H3”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                </w:headers>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>One Child</w:t>
                </w:r>
          </w:p>
    </w:tc>
    <w:tc w:id= “H4”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                </w:headers>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>Two Children</w:t>
                </w:r>
          </w:p>
    </w:tc>
</w:tr>
<w:tr>
    <w:tc w:id= “H5”>
          <w:tcPr>
                <w:colSpan w:val="3"/>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>Married filing jointly and you have -</w:t>
                </w:r>

<!-- page 4527 -->

Annex J

4517
          </w:p>
    </w:tc>
</w:tr>
<w:tr>
    <w:tc w:id= “H2”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                      <w:header w:val="H2"/>
                      <w:header w:val="H5"/>
                </w:headers>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>$100</w:t>
                </w:r>
          </w:p>
    </w:tc>
    <w:tc w:id= “H3”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                      <w:header w:val="H3"/>
                      <w:header w:val="H5"/>
                </w:headers>
          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>One Child</w:t>
                </w:r>
          </w:p>
    </w:tc>
    <w:tc w:id= “H4”>
          <w:tcPr>
                <w:headers>
                      <w:header w:val="H1"/>
                      <w:header w:val="H4"/>
                      <w:header w:val="H5"/>
                </w:headers>

<!-- page 4528 -->

ECMA-376 Part 1

4518

          </w:tcPr>
          <w:p>
                <w:r>
                      <w:t>Two Children</w:t>
                </w:r>
          </w:p>
    </w:tc>
</w:tr>
The cell with $200 declares three headers:

H1 – Married filing jointly and you have -
H3 – One Child
H5 – Your Credit Is-
J.4.6
Delivering Predictable Navigation within Forms
Although it is often the case that navigating documents is simple by virtual of the natural flow of
documents, it can become complex in the case of forms being found in the document.  It is highly
desirable for document and template authors to define those forms such that the navigation from
control to control is both intelligent and predictable.  Office Open XML provides a mechanism for
defining the navigation flow ( also known as tab order ) for such controls.
Consider part of a well known form:

The following mark up can be used to make the form controls on that line accessible:
<w:p>
    <w:sdt>
        <w:sdtPr>
            <w:id w:val="1" />
        </w:sdtPr>
        <w:sdtContent>
            <w:sdt>
                <w:sdtPr>
                    <w:id w:val="2" />
                </w:sdtPr>
                <w:sdtContent>
                    <w:r>
                        <w:t>Number of qualifying children:</w:t>

<!-- page 4529 -->

Annex J

4519
                    </w:r>
                </w:sdtContent>
            </w:sdt>
            …
            <w:r>
                …
                <w:fldChar w:fldCharType="begin">
                    <w:ffData>
                        <w:name w:val="NumberOfChildren" />
                        <w:enabled />
                        …
                        <w:textInput />
                        <w:label w:val="2" />
                        <w:tabIndex w:val="1" />
                    </w:ffData>
                </w:fldChar>
            </w:r>
            …
            <w:r>
                <w:t xml:space="preserve"> X $1,000</w:t>
            </w:r>
        </w:sdtContent>
    </w:sdt>
</w:p>
<w:p>
  …
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="3" />
    </w:sdtPr>
    <w:sdtContent>
      <w:r>
        <w:t>Enter the result</w:t>
      </w:r>
    </w:sdtContent>
  </w:sdt>
</w:p>
<w:tbl>
  …
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="4" />
    </w:sdtPr>

<!-- page 4530 -->

ECMA-376 Part 1

4520

    <w:sdtContent>
      <w:tr>
        …
        <w:tc>
          …
          <w:p>
            <w:r>
              <w:t>1</w:t>
            </w:r>
          </w:p>
        </w:tc>
        <w:tc>
          …
          <w:p>
            <w:r>
              <w:fldChar w:fldCharType="begin">
                <w:ffData>
                  <w:name w:val="Line1Value" />
                  <w:enabled />
                  …
                  <w:textInput />
                  <w:label w:val="1" />
                  <w:label w:val="3" />
                  <w:tabIndex w:val="2" />
                </w:ffData>
              </w:fldChar>
            </w:r>
            …
          </w:p>
        </w:tc>
      </w:tr>
    </w:sdtContent>
  </w:sdt>
</w:tbl>
The first line of this form is broken up into two Form Text Fields and four content controls that are used
as labels. The Form Text Field with a name of NumberOfChildren is associated in terms of its label with
the content control that has an id value of 2. In this case, the Form Text Field with a name of
NumberOfChildren has a label of Number of qualifying children:.  According to the tabIndex element the
first Form Text Field is navigated first.

<!-- page 4531 -->

Annex J

4521
J.4.7
Hierarchical Labels in Content Controls
It is often the case that content controls, for example, are used together such that they create
hierarchical relationships.  Labels can, and should be used, to clarify exactly what the intent is for each
content control.
Consider the following collection of content controls:
Sender:
Home Address:

Address 1:
1 Cherry Lane
<w:p>
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="55303308"/>
      <w:text/>
    </w:sdtPr>
    <w:sdtContent>
      <w:r>
        <w:t>Sender</w:t>
      </w:r>
    </w:sdtContent>
  </w:sdt>
  <w:r>
    <w:t>:</w:t>
  </w:r>
</w:p>
<w:p>
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="55303309"/>
      <w:text/>
    </w:sdtPr>
    <w:sdtContent>
      <w:r>
        <w:t>Home Address</w:t>
      </w:r>
    </w:sdtContent>
  </w:sdt>
  <w:r>
    <w:t>:</w:t>
  </w:r>

<!-- page 4532 -->

ECMA-376 Part 1

4522

</w:p>
<w:p>
  <w:r>
    <w:tab/>
  </w:r>
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="55303310"/>
      <w:text/>
    </w:sdtPr>
    <w:sdtContent>
      <w:r>
        <w:t>Address 1</w:t>
      </w:r>
    </w:sdtContent>
  </w:sdt>
  <w:r>
    <w:t>:</w:t>
  </w:r>
  <w:r>
    <w:tab/>
  </w:r>
  <w:sdt>
    <w:sdtPr>
      <w:id w:val="55303307"/>
      <w:label w:val="55303308"/>
      <w:label w:val="55303309"/>
      <w:label w:val="55303310"/>
    </w:sdtPr>
    <w:sdtContent>
      <w:r>
        <w:t>1 Cherry Lane</w:t>
      </w:r>
    </w:sdtContent>
  </w:sdt>
</w:p>
In this example there are three controls: one for the sender, one for home address and one for the first
line of the address. The labels are nested, from most generic to most specific.

<!-- page 4533 -->

Annex J

4523
J.5
Best Practices for Customers of Office Open XML Implementations
J.5.1
Using Common Accessibility Features
This version of the Office Open XML specification contains a rich set of primitives to expose additional
and/or alternative data for many objects that can be included in such documents.  Customers are
encouraged to make sure of each of these when possible.
It is acknowledged that there are consistency issues with these primitives; generating consistency will be
handled by way of maintenance.
J.5.1.1
VML Objects
Even when working with VML objects, customers of the Office Open XML specification can ensure that
their documents take advantage of common accessibility features such as captions and long
descriptions.
Consider the camera feature in SpreadsheetML.  Although the camera is implemented using VML,
customers can still assign captions and descriptions.
<v:shape id="_x0000_s1025" type="#_x0000_t75" alt="A camera object goes
here"
  title=”Stock Sales” style='position:absolute;margin-left:237.75pt;margin-
top:79.5pt;width:96.75pt;
  height:45.75pt;z-index:1' filled="t" fillcolor="white [9]" stroked="t"
  strokecolor="windowText [64]" o:insetmode="auto">
  <v:imagedata o:relid="rId1" o:title=""/>
  <x:ClientData ObjectType="Pict">
    <x:SizeWithCells/>
    <x:Anchor> 4, 61, 5, 6, 6, 62, 8, 7</x:Anchor>
    <x:FmlaPict>$A$1:$B$3</x:FmlaPict>
    <x:CF>Pict</x:CF>
    <x:AutoPict/>
    <x:Camera/>
  </x:ClientData>
</v:shape>
In the above mark up, the title attribute plays the traditional role of caption and the alt attribute plays
the traditional role of long description.
J.5.1.2
External MLs
In situations where customers find themselves adding content to their documents using MLs that are
not part of Office Open XML, they can still take advantage of the built-in accessibility primitives.
Consider the case where the customer elects to embed some external mark-up on a slide via the
contentPart.

<!-- page 4534 -->

ECMA-376 Part 1

4524

<p:spTree>
  …
  <p:grpSp>
    <p:nvGrpSpPr>
      <p:cNvPr id="5" name="Group 4" descr="This object contains the math
equations, expressed as MathML, that dictate water flow over the surface of
the vehicle."
        title="Waterflow Equation"/>
        <p:cNvGrpSpPr/>
      <p:nvPr/>
    </p:nvGrpSpPr>
    <p:grpSpPr>…</p:grpSpPr>
    <p:contentPart r:id="smil01">…</p:contentPart>
  </p:grpSp>
</p:spTree>
J.5.2
Using Explicit Structure When Possible
When authoring long or complex documents, structure can substantially improve the consumption
experience for all readers. This is particularly true for those readers who make use of our accessibility
features.  For these readers, it is highly recommended that customers make use of many of the built-in
document structure primitives found in Office Open XML.
Examples of such primitives include page and section breaks, lists, headings, headers and footers and
even frames.
J.5.3
Creating Accessible Animations
An Office Open XML-recommended approach for providing text equivalents for animations is to create a
separate text box for each animation.  Each text box should contain the desired closed caption for the
animation.  The exact specifics would naturally be left to the document author, but could include
information such as a description of the object being animated, the type of animation, the visual effects,
etc.
The text boxes that contain the text equivalent per animation would be arranged on the slide in a
desired location; using the closed captioning model, centered along the bottom third of the screen is a
standard example.  Once positioned properly, the text boxes would be put on the animation timeline
along with their animation, such that they appear when the animation starts and that they disappear
when the animation concludes.
Consider a shape that is set to move vertically along the left-hand side of the slide.  To provide a text
equivalent for this animation, I would add a text box for the move animation.  The slide may look like
this:

<!-- page 4535 -->

Annex J

4525

In this scenario, ythere are two shapes on the slide, which will be animated.  Both of these shapes can
be found in the slide’s shape tree:
<p:spTree>
  <p:nvGrpSpPr>…</p:nvGrpSpPr>
  <p:grpSpPr>…</p:grpSpPr>
  <p:sp>
    <p:nvSpPr>
      <p:cNvPr id="5" name="Rectangle 4" descr="Blue rectangle of height
1.5 inches and width 2 inches with text “Shape  1”"
   title=”Blue Rectangle”/>
        <p:cNvSpPr/>
      <p:nvPr/>
    </p:nvSpPr>
    <p:spPr>
      <a:xfrm>…</a:xfrm>
      <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
      …
    </p:spPr>
    <p:style>…</p:style>
    <p:txBody>
      …
      <a:p>
        <a:pPr algn="ctr"/>
        <a:r>

<!-- page 4536 -->

ECMA-376 Part 1

4526

          <a:rPr lang="en-US" dirty="0" smtClean="0"/>
                     <a:t>Shape 1</a:t>
        </a:r>
        <a:endParaRPr lang="en-US" dirty="0"/>
      </a:p>
    </p:txBody>
  </p:sp>
  <p:sp>
    <p:nvSpPr>
      <p:cNvPr id="8" name="TextBox 7" descr="Caption for
animations:&#xA;Blue rectangle of height 1.5 inches and width 2 inches with
text “Shape 1” is moving from bottom left of slide to top left of slide
while fading in over 3 seconds.&#xA;"
        title=”Animation Close Caption”/>
      <p:cNvSpPr txBox="1"/>
      <p:nvPr/>
    </p:nvSpPr>
    <p:spPr>
      <a:xfrm>…</a:xfrm>
      <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
      <a:noFill/>
    </p:spPr>
    <p:txBody>
      <a:bodyPr wrap="square" rtlCol="0">
        <a:spAutoFit/>
      </a:bodyPr>
      <a:lstStyle/>
      <a:p>
        <a:r>
          <a:rPr lang="en-US" dirty="0" smtClean="0"/>
            <a:t>Blue rectangle of height 1.5 inches and width 2 inches
with text “Shape 1” is moving from bottom left of slide to top left of
slide.</a:t>
        </a:r>
        <a:endParaRPr lang="en-US" dirty="0" smtClean="0"/>
      </a:p>
    </p:txBody>
  </p:sp>
</p:spTree>
As the shape tree above shows, the animating shape has a shape ID of 5 and the closed caption text box
has a shape ID of 8.  With both shapes in the shape tree, they can be added to the slide timeline.

<!-- page 4537 -->

Annex J

4527
<p:timing>
  …
  <p:bldLst>
    <p:bldP spid="5" grpId="0" animBg="1"/>
    <p:bldP spid="8" grpId="1"/>
    <p:bldP spid="8" grpId="2"/>
  </p:bldLst>
</p:timing>
There are three build steps in the timeline:
Animate the shape ( move across the slide )
Animate the text box ( appear )
Animation the text box  ( disappear )
This approach can be used in even more complex animations by reusing the general principle of
associating a text box per animation.
J.5.4
Specifying Language to Enable Accessibility Tools
It is incredibly important for customers to always specify which language is being used to author a
document.  The rationale behind this is that speech synthesizers and Braille devices can automatically
switch to the new language, making the document more accessible to multilingual users.
when an Office Open XML document uses different languages for text, a clear identification of the
language used and its country is provided. The <w:lang/> tag is generated which indicates the language
used in the paragraph and run.
<w:p>
  <w:r w:rsidR="0008256F">
    <w:t xml:space="preserve">multiple languages like </w:t>
  </w:r>
  <w:r w:rsidR="0008256F">
    <w:t>شقشالهؤ سشعيه شقشالهس</w:t>
  </w:r>
  <w:r w:rsidR="0008256F">
    <w:rPr>
      <w:rFonts w:hint="cs"/>
      <w:rtl/>
      <w:lang w:bidi="ar-DZ"/>
    </w:rPr>
    <w:t>شقشالهؤ شملثقهش</w:t>
  </w:r>

<!-- page 4538 -->

ECMA-376 Part 1

4528

  <w:r w:rsidR="0008256F">
    <w:rPr>
      <w:rFonts w:hint="cs"/>
      <w:rtl/>
      <w:lang w:bidi="ar-EG"/>
    </w:rPr>
    <w:t>شق
شالهؤ ثلغحف</w:t>
  </w:r>
</w:p>
J.5.5
Using Styles when Possible
Although most people associate styles usage only with the goal of making a document look visually
appealing, they can also be of particular use for those readers who are trying to read documents.  If a
customer authors a document such that it uses styles, the transform a reader can do to accommodate a
visual impairment is substantially easier than the transform required for documents not based on a
style: in the style case, a styles optimized for visual impaired readers can be quickly applied; in the non-
style case, extensive direct formatting of the document must be done by the reader.
J.5.6
Techniques for Improving Document Navigation
It is highly desirable for customers to author their documents in such a way that there is predictability in
terms of navigation.  The following describes the techniques that the customer can use to make use
built-in primitives in Office Open XML, as well as how such techniques change the underlying document
data, to achieve the desired outcome.
J.5.6.1
Navigating Shapes on Slides
In cases where a reader is navigating slides, navigation is complicated by the 2D-nature of the slide.
Office Open XML provides a remedy for this by allowing customers to define the reading order, or
navigation order, of shapes on a slide through the use of z-order.
Consider the case that a presentation author elects to construct a slide that contains five placeholders:
one for the title (A) , two for list headings (B, C) and two lists (D, E).  Visually, this could be represented
as follows:

<!-- page 4539 -->

Annex J

4529

Further, suppose that the default tab order (aka reading order) for such a slide, in a LTR-orientation, is A,
B, D, C, E.
In DrawingML, this ordering is accomplished by the lexical ordering of the five shapes, A – E.  In the
shape tree for this slide, you would see this:
<p:spTree>
  <p:nvGrpSpPr>…</p:nvGrpSpPr>
  <p:grpSpPr>…</p:grpSpPr>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>A</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" dirty="0" smtClean="0"/>
        <a:t>B</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>

<!-- page 4540 -->

ECMA-376 Part 1

4530

    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>D</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" dirty="0" smtClean="0"/>
        <a:t>C</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>E</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
</p:spTree>
If this default reading order was not what the presentation author preferred, the author could specify
the z-order for each shape to determine the ordering of the shapes in the shape tree, and by extension,
the ordering that shapes would be read in the presentation.  The shapes will show up in the shape tree
starting with the shape farthest “back” and ending with the shape in the “front.”
Using our example above, if the author wanted to change the ordering of the shapes to facilitate a more
RTL-like reading order ( A, C, E, B, D ), she could select shape E and send it to the back, then select shape
C and sent it to the back and finally select shape A and send it to the back.  Naturally the user experience
here is arbitrary; the important aspect is that the ordering of the shapes within the shape tree is
changed to the preferred reading order.  With these three changes, the shape tree is changed to this:
<p:spTree>
  <p:nvGrpSpPr>…</p:nvGrpSpPr>
  <p:grpSpPr>…</p:grpSpPr>
  <p:sp>
    …

<!-- page 4541 -->

Annex J

4531
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>A</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" dirty="0" smtClean="0"/>
        <a:t>C</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>E</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" dirty="0" smtClean="0"/>
        <a:t>B</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>
  <p:sp>
    …
    <p:txBody>…
      <a:r>
        <a:rPr lang="en-US" sz="4800" b="1" dirty="0" smtClean="0"/>
        <a:t>D</a:t>
      </a:r>…
    </p:txBody>
  </p:sp>

<!-- page 4542 -->

ECMA-376 Part 1

4532

</p:spTree>
J.5.7
Creating Image Map-Like Functionality
Although Office Open XML does not natively support image maps via a specific tag, the technology
contained within Office Open XML can be used to deliver image map-like functionality.  In short, to
deliver image map-like functionality, four components are needed: an object to “map,” a clickable
region, hit testing and a hyperlink.
By using an image as the object to be “mapped,” one can position no-fill, no-line DrawingML shapes
over that image to define the clickable regions as well as to provide hit testing.  It is often suggested that
the shapes be grouped with the object, although that is not strictly required.  A hyperlink per shape can
be used to provide the actual hyperlink itself.
Consider the case where a slide has a picture that includes two objects, and the author wants to provide
image map-like functionality. She can position shapes “over” the picture to get the desired functionality.
This construction appears as follows:

Image file to be “mapped”

Shapes positioned over interactive areas
The following XML shows the three shapes in the shape tree: one for the image and two, one for each of
the shapes that are positioned over the image.
<p:spTree>
<p:nvGrpSpPr>
<p:cNvPr id="1" name=""/>
<p:cNvGrpSpPr/>
<p:nvPr/>
</p:nvGrpSpPr>
<p:grpSpPr>
<a:xfrm>...</a:xfrm>
</p:grpSpPr>
// This represents the “mapped” object
<p:pic>
<p:nvPicPr>

<!-- page 4543 -->

Annex J

4533
<p:cNvPr id="4" name="Picture 3" descr="A picture from our 2007 trip to
Pisa, Italy" title="Classic Pisa"/>
<p:cNvPicPr>...</p:cNvPicPr>
<p:nvPr/>
</p:nvPicPr>
<p:blipFill>
// The rel ID refers to the image that makes up the object
<a:blip r:embed="rId2"/>
<a:stretch><a:fillRect/></a:stretch>
</p:blipFill>
<p:spPr>...</p:spPr>
</p:pic>
// This represents the first shape positioned over the one of the
// “mapped” regions of object above.
<p:sp>
<p:nvSpPr>
// The hyperlink contains both the tooltip that provides
// hit-testing feedback is defined in the tooltip attribute;
// the hyperlink is defined by the relationship ID attribute.
<p:cNvPr id="5" name="Rectangle 4" descr="Rachel">
<a:hlinkClick r:id="rId3" tooltip="More information about Rachel"/>
</p:cNvPr>
<p:cNvSpPr/>
<p:nvPr/>
</p:nvSpPr>
<p:spPr>
<a:xfrm>...</a:xfrm>...
// The shapes to trigger the hyperlink and hit testing
// have no fill and no line applied to them.
<a:noFill/>
<a:ln><a:noFill/></a:ln>
</p:spPr>
<p:style>...</p:style>
<p:txBody>...</p:txBody>
</p:sp>
// This is the second shape positioned over the picture to provide hit
testing and hyperlink behavior
<p:sp>...
</p:sp>
</p:spTree>

<!-- page 4544 -->

ECMA-376 Part 1

4534

<Relationships
xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId3"
Type="http://purl.oclc.org/ooxml/officeDocument/relationships/hyperlink"
Target="http://url2/" TargetMode="External"/>
    <Relationship Id="rId2"
Type="http://purl.oclc.org/ooxml/officeDocument/relationships/image"
Target="../media/image1.jpeg"/>
    <Relationship Id="rId1"
Type="http://purl.oclc.org/ooxml/officeDocument/relationships/slideLayout"
Target="../slideLayouts/slideLayout7.xml"/>
    <Relationship Id="rId4"
Type="http://purl.oclc.org/ooxml/officeDocument/relationships/hyperlink"
Target="http://url1/" TargetMode="External"/>
</Relationships>
J.5.8
Tables
There are two types of tables in Office Open XML: tables stored as WordprocessingML and tables stored
as DrawingML.  Both tables enable customers to provide captions and descriptions for their tables.
Customers are strongly encouraged to ensure that all tables contain both captions and descriptions.
J.5.8.1
Using Captions and Long Descriptions in WordprocessingML Tables
Consider a simple table that contains both captions and long descriptions.
<w:body>
  <w:tbl>
    <w:tblPr>
      <w:tblStyle w:val="TableGrid"/>
      <w:tblW w:w="0" w:type="auto"/>
      <w:tblLook w:firstRow="true" w:firstColumn="true"
        w:noVBand="true"/>
      <w:tblCaption w:val="2004 Figures"/>
      <w:tblDescription w:val="This table shows that sales figure have been
rising for the past five quarters."/>
    </w:tblPr>
    …
  </w:tbl>
  …
</w:body>
J.5.8.2
Using Captions and Long Descriptions in DrawingML Tables
Consider a simple table that contains both captions and long descriptions.

<!-- page 4545 -->

Annex J

4535
<p:graphicFrame>
  <p:nvGraphicFramePr>
    <p:cNvPr id="4" name="Table 3" title=”2004 Figures”
      descr=”This table shows that sales figures have been rising for the
past five quarters.”/>
    …
  </p:nvGraphicFramePr>
  <p:xfrm>…</p:xfrm>
  <a:graphic>
    <a:graphicData uri="…">
      <a:tbl>
        …
      </a:tbl>
    …
    </a:graphicData>
  </a:graphic>
</p:graphicFrame>
End informative annex.
