# J-CARD design, editing and printing

[Home](../README.md) · [User guide](usage.md) · [日本語](jcard.ja.md)

A J-CARD is the paper insert for a cassette case. Initial layout, editing, saving and printing instructions are kept together here.

## 1. Create from a cassette

Select a cassette and choose **Edit J-CARD**.

- **Outside**: linked text boxes receive the title, A/B track titles, artists and recorded durations.
- **Inside**: starts blank and white. Refreshing track lists does not add boxes to it.
- Existing saved artwork opens unchanged. Audio and track order are unaffected.
- Defaults are front 65 mm, spine 12 mm, flap 25 mm and height 100 mm. Measure your case and adjust them.

![Outside: cassette title and A/B track lists](screenshots/jcard-outside-en.jpg)

## 2. Switch outside and inside

Use **Editing side**. Manually add text/images inside if desired. Text remains readable rather than mirrored; only fold positions are mirrored. Both PDF faces use matching sheet positions.

![Inside: blank on a new card](screenshots/jcard-inside-en.jpg)

## 3. Edit text, images and rectangles

1. Use **Add text**, **Add image** or **Add rectangle**.
2. Select on the canvas or in the left elements list, then drag to move.
3. Set position, dimensions, rotation and color in the right pane. Text supports family, size, alignment, line spacing, bold and vertical writing.

| Action | How |
| --- | --- |
| Resize | Drag corner/edge squares; font size stays fixed and text reflows |
| Rotate | Drag the top circle; hold Shift for 15-degree steps |
| Move | Drag, or use arrows on the canvas (0.5 mm; Shift: 5 mm) |
| Align | Left-pane snapping, center controls or numeric properties |
| Stack, duplicate, delete | Top toolbar |
| Undo | Undo/redo; Esc cancels an active drag |

Resizing a rotated box keeps its opposite anchor fixed. Zoom/snapping are in the left pane; handle instructions appear above Color in the right pane.

## 4. Update changed track lists

After editing the cassette, choose **Refresh track lists**. It reads the latest saved mix and updates **linked boxes only**, retaining placement, formatting and rotation.

- Default outside lists are already linked.
- For a custom box, choose title, spine or SIDE A/B under **Linked content**.
- Choose **Free text (do not update)** to keep independent wording. Editing text alone does not unlink it.
- Inside boxes you deliberately link can be refreshed too; an empty inside stays empty.
- Original legacy list boxes with cleared bindings are recognized.

## 5. Move or copy between faces

Select an element, choose **Cut**, switch face, then **Paste**. **Copy** duplicates instead. Styling, rotation and links are retained, with placement on the corresponding panel.

Ctrl/Cmd+X, C and V are supported. These use an in-app J-CARD clipboard. Cutting/copying selected text inside input fields remains a separate native text operation.

## 6. Set dimensions, panels and paper

The left pane controls dimensions, A4/Letter and margins. Changing dimensions does not scale existing artwork.

Choose Standard (12 mm) or Custom for the spine width. For slim cases, enter the measured width using Custom. Existing J-CARD spine widths are preserved.

Fold-out panels are added to the right of the front. Widths range from 10 to 100 mm. The card must fit **one sheet at actual size**, including 5 mm of guide space on each edge and the chosen paper margin.

Default dimensions with 5 mm margins allow two extra 65 mm panels on A4 or Letter. Unfolded widths are 102/167/232 mm for +0/+1/+2. Narrower panels may permit more; the limit is geometric. Separate-sheet booklets are unsupported.

Outside artwork stays in place; inside artwork shifts relative to the original front/spine/flap. Elements on removed panels remain, so move them as needed.

## 7. Save and reopen artwork

Artwork autosaves per cassette; status is at the top right. **Close** waits for saving. Failed saves keep the editor open and retain edits. Reopen the same cassette's **Edit J-CARD** to continue.

**Download editable file** exports a portable `.jcard.json`. **Open editable file** reopens an earlier design or reuses it on another cassette. Images, both faces, panels and paper settings are included.

Autosaved files are under `jcards` in the data directory; see [storage and backup](usage.md#storage). Deleting a cassette retains its artwork file, but that deleted cassette cannot open it.

## 8. Export SVG/PDF and print

| Output | Contents |
| --- | --- |
| SVG | Current face at physical size, with optional fold lines/outline |
| PDF | Two pages, outside/inside at matching A4/Letter positions; orientation chosen to fit |
| PDF guides | Long dashed cutting lines/corner marks and short dashed fold lines/fold marks |

Print at **Actual size / 100%**, without shrinking. For duplex, start with long-edge binding in portrait or short-edge binding in landscape, then test dimensions/alignment.

PDF embeds a 300 dpi image to preserve Japanese appearance and omits selection outlines/handles. It is not a selectable vector-text PDF. SVG text remains editable, but other software's fonts may alter appearance.

## 9. Editing limits

Sans-serif, serif and monospace families use locally available fonts. Limits: 5 MB per PNG/JPEG, 150 elements per face and 12 MB per save request.

Freehand paths, image cropping, mixed formatting inside one text box and DOCX are unsupported. Approximate overflow warnings are not shown. Checks are listed in the [development guide](development.md).
