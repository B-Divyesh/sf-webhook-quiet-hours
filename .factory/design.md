# Webhook Quiet Hours — visual thesis

## Direction: a nocturnal botanical field guide

Webhook operations are treated like field observation: many specimens arrive,
related forms are pressed into one fingerprint, and only the dangerous ones are
flagged. The interface is a night botanist's working folio rather than a generic
SaaS dashboard. Ruled baselines, specimen numbers, marginal annotations and a
single hand-painted moon-bloom give the operational data a memorable logic.
Decoration always explains aggregation, quiet time or severity.

## Palette

- `night-950` #152019 — ink-dark forest background; primary dark surface.
- `night-850` #203027 — elevated dark surface.
- `paper-50` #F4F0E5 — warm herbarium paper; light background and dark-mode text.
- `paper-150` #E6DECB — rules and pressed-paper surfaces.
- `ink-900` #172019 — body ink on paper.
- `moss-650` #3F654B — primary action; selected healthy specimen.
- `moss-750` #2F513B — action hover with white text at accessible contrast.
- `lichen-500` #8AA174 — secondary status.
- `amber-650` #9A641C — warning with an explicit label.
- `berry-700` #A33B38 — high-severity flags and destructive actions.
- `sky-700` #2C6373 — links and focus-adjacent information.

Light mode is the default working folio. Dark mode turns the folio into a
night-survey desk, preserving paper as the high-contrast ink color. State is
always conveyed by icon/word as well as color.

## Type and rhythm

Two local/system families, no network fonts: Georgia for specimen headings and
system UI (`ui-sans-serif`, Inter fallback) for controls and dense operational
copy. The contrast feels archival but remains fast and familiar. Scale: 14, 16,
20, 25, 39 px; body is never below 16 px. Data uses tabular figures. Text
measures are capped at 72 characters. Spacing follows a 4/8 px rhythm with
8, 12, 16, 24, 32, 48 and 72 px steps.

## Interaction grammar

- An alias is a labelled specimen; fingerprints are observations grouped under it.
- Severity uses small tied-paper flags with a word label, never color alone.
- Expansion opens an observation below its row, preserving spatial context.
- The primary action is always one moss-filled control. Secondary actions are ink outlines.
- Controls are at least 44 px; focus is a 3 px amber/ink double-ring.
- At 390 px, the marginalia and illustration recede, tables become stacked specimen records,
  and the persistent action area becomes ordinary document flow.

## Motion

New observations settle with a 180 ms opacity/translate transition; disclosures
open in 220 ms. The hero's drawn route is static—nothing loops. With
`prefers-reduced-motion`, all transforms and smooth scrolling are removed and
state changes are instant. Motion never carries unique information.

## Original asset plan and provenance

The hero asset is a generated field-guide plate: a moon-bloom whose clustered
seed capsules resolve into one red flagged specimen. It explains compression,
not capability. It is displayed as a contained plate with an explicit caption.

Prompt sheet: “Nineteenth-century botanical field guide plate of an imaginary
nocturnal webhook plant, branching dark green stems carry many tiny sealed seed
capsules that converge into one distinctive crimson warning berry, pressed
herbarium paper, fine engraved ink lines with restrained watercolor washes,
deep forest green, lichen, parchment, oxide red, quiet moonlit scientific
observation, centered specimen with generous negative space, orthographic plate,
no people, no insects, no text, no letters, no numbers, no watermark, no logos,
no interface, no photorealism.”

- Generator: Azure AI Foundry factory image deployment via
  `/opt/fleet/lib/gen-image.sh`.
- Date: 2026-08-27.
- License/provenance: original AI-generated asset made for this product; no
  brands, real people or copyrighted characters requested.
- Source PNG and exact prompt sidecar live in `assets/src/`; optimized WebP is
  shipped locally. Footer discloses generated imagery.
- The 1200×630 Open Graph image and 180×180 Apple touch icon are mechanical
  crops of that same original plate, made locally with ImageMagick on
  2026-08-30. They introduce no new generated source or third-party asset.

The demo uses the same folio rather than a separate marketing treatment. Its
dark ink banner stays visible across every sample view, and its two controls
follow the existing square, labelled action grammar. This makes temporary
state clear without weakening the product-specific field-guide identity.

## Performance and accessibility

The hero WebP must be at most 300 KB with explicit dimensions. No runtime CDN,
font download or ornamental JavaScript. Both color treatments meet WCAG AA for
body copy; focus, error text and interactive outlines meet 3:1. One semantic
`h1`, landmark structure, labelled forms, announced async states, and text
alternatives are required.
