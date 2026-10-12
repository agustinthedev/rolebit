# Rolebit design system

## Direction

Rolebit uses a premium light desktop aesthetic: pearlescent off-white space, restrained lavender/peach/mint aurora light, softly elevated ceramic-glass surfaces, dark violet typography, compact technical labels, and sparse semantic color. It intentionally avoids generic admin-dashboard styling, saturated gradients, dark cyberpunk motifs, and decorative motion that competes with job data.

The source of truth is the token block in `src/App.css`.

## Color

| Role | Value |
|---|---|
| Base background | `#F6F5FB` |
| Warm variation | `#FAF8FC` |
| Cool variation | `#F0F3FB` |
| Main surface | translucent `#FFFFFF` |
| Primary text | `#211D3D` |
| Secondary text | `#747187` |
| Muted metadata | `#9A98AD` |
| Accent purple | `#7165DD` |
| Success mint | `#159C89` |
| Error coral | `#DF7183` |
| Informational blue | `#6598CA` |

Aurora colors use lavender `#D9D2FA`, violet `#EAE0FA`, pink `#F6DDE9`, peach `#FBE7DC`, cyan `#D9F2F3`, and mint `#D5EFE8` as large blurred radial fields. They never become the dominant reading surface.

## Typography

The stack begins with Segoe UI Variable for native Windows rendering and falls back to Segoe UI, Inter, and the system sans-serif. One family is used throughout.

- Page title: 27–36px, 760 weight, tight tracking.
- Major metric: 34–47px, 750 weight, tabular numerals.
- Card title: 15–20px, 600–700 weight.
- Body: 12–14px with generous line height.
- Technical label: 9–10px uppercase monospace, increased tracking.

Descriptions and job content never use monospace. Query strings, IDs, and compact metadata may.

## Geometry and spacing

Spacing follows an 8px rhythm with optical adjustments at dense table boundaries.

- Major panel radius: 28–34px.
- Standard card radius: 26px.
- Compact panel radius: 18–20px.
- Control radius: 13–14px.
- Main content padding: 25–44px depending on viewport.
- Standard panel padding: 20–27px.

Cards use a broad low-opacity violet shadow, a close secondary shadow, a translucent white border, and a one-pixel inset highlight. Glass blur is reserved for navigation, major surfaces, dialogs, and notices; inputs remain nearly opaque.

## Components

- `AppShell`: integrated translucent sidebar and fixed aurora background.
- `PageHeader`: technical eyebrow, title, description, and contextual actions.
- `SurfaceCard`: shared surface depth and radius.
- `Button` and `IconButton`: primary, secondary, ghost, and danger hierarchy.
- `StatusBadge`: semantic pill with a dot so color is not the sole cue.
- `Field`, `Input`, `Select`, and `Switch`: consistent validation and focus treatment.
- `SearchListEditor`: reusable strategy list editor.
- `Modal`: Radix Dialog behavior with spacious desktop presentation.
- `EmptyState`, `Skeleton`, `InlineNotice`, and progress components: intentional non-happy paths.

## Status semantics

- Purple: new, updated, running, selected.
- Mint: saved, completed, active, reopened, successful.
- Coral: failed, cancelled, dismissed, closed.
- Amber: partial, interrupted, unknown, warning.
- Blue: seen and informational state.

Every status includes text and, where compact, a shape marker.

## Motion

Direct controls transition in 120–180ms. Pages and dialogs enter within 180–220ms. Progress width changes use 260ms easing. There are no continuous decorative animations; the loading spinner is functional. `prefers-reduced-motion` reduces all animation and transition durations.

## Responsive behavior

The primary targets are 1280×800, 1440×900, and 1920×1080. Below 1080px the sidebar compacts to icons, metric cards reflow, secondary columns stack, and dense tables retain horizontal scrolling. The desktop window enforces a 1050×700 minimum to protect core workflows. Dialogs cap their size to the viewport and scroll internally.

## Accessibility

- Normal text targets WCAG AA contrast despite pastel surroundings.
- Keyboard focus uses a visible purple ring and offset.
- Native or Radix semantics back interactive controls.
- Buttons include textual names; icon-only controls have accessible labels.
- Status is never communicated by color alone.
- Reduced motion is respected.
- Table headers remain distinct and sticky inside overflow surfaces.
