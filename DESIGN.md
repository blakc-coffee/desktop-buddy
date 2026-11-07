---
name: Desktop Buddy
description: A native mini pet and command bar with optional warm charcoal utility tools and pet-specific accent colors.
colors:
  primary: "#b9e59b"
  primary-text: "#1f3020"
  canvas: "#1c1c1a"
  surface: "#232320"
  raised: "#2e2e2b"
  hover: "#35352f"
  text: "#f5f4ef"
  muted: "#b8b7ad"
  faint: "#9d9c91"
  border: "#42423b"
  error: "#ffb4a8"
  light-primary: "#315c25"
  light-primary-text: "#f4faef"
  light-canvas: "#f7f7f2"
  light-surface: "#eeeee7"
  light-raised: "#e5e5dc"
  light-hover: "#deded2"
  light-text: "#24251e"
  light-muted: "#53574a"
  light-faint: "#626759"
  light-border: "#c7cabd"
  light-error: "#a32319"
typography:
  headline:
    fontFamily: "Segoe UI, system-ui, sans-serif"
    fontSize: "27px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Segoe UI, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.5
  body:
    fontFamily: "Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Segoe UI, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
rounded:
  control: "7px"
  capture: "8px"
  panel: "10px"
  tag: "4px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-text}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
  input:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "9px 11px"
  navigation:
    textColor: "{colors.muted}"
    rounded: "{rounded.control}"
    padding: "11px 13px"
  status-tag:
    textColor: "{colors.muted}"
    rounded: "{rounded.tag}"
    padding: "2px 7px"
---

# Design System: Desktop Buddy

## Native toolbar and compact Todo

The native command bar uses Segoe UI at 14 DIP, a 56 DIP input row, 48 DIP results, a reserved Actions text button, and the selected pet's semantic palette. It opens blank; suggestions appear only while browsing. Native result icons use a consistent drawn outline. Actions groups detailed tools rather than exposing a long flat menu. The pet context menu has five entries.

The compact Todo window uses the same pet palette, a 24px Todo heading, a single add row and divider-separated editable checklist rows. It remains a desktop surface with 300px minimum width. Detailed management keeps the established warm charcoal design below.

## Overview

The warm-charcoal reference remains the base. The panel uses system typography, flat tonal surfaces, divider-led lists and one accent matched to the chosen pet: apricot for Calico, amber for Dog, mint for Panda. Light mode retains the component structure with its own semantic palette; system mode follows the OS color preference.

This documents the current React/CSS implementation. No pet, animation loop, gradients, remote fonts or decoration libraries belong to the approved direction.

**Key Characteristics:**

- Quiet surfaces and clear next actions.
- Persistent quick capture and list-based content.
- Compact Windows panel behavior rather than a separate mobile product.

## Colors

### Primary

Restrained mint identifies primary actions, selected navigation, active tabs, focus, timer values and useful status marks. Light mode substitutes a deep green accent and pale action text.

### Neutral

Warm charcoal canvas, surface and raised tones provide structure. Warm off-white primary text, muted descriptions and faint metadata establish hierarchy. Borders divide rows and outline controls. Light equivalents retain that hierarchy on pale warm surfaces. Error tokens mark errors and destructive-action hover feedback.

## Typography

**Display Font / Body Font:** Segoe UI with system-ui and sans-serif fallbacks. No downloaded font. Page headlines use the headline role, reducing to (23px) at the first compact breakpoint. Section titles use the title role; tertiary headings use (15px, weight 500). Labels and metadata use the label role; some explanations use (13px).

Body content uses the body role. Explanations and resume notes cap lines at (72ch). Countdowns use tabular numerals at (23px), reducing to (20px) in the narrow panel. Paths and long titles wrap; resume notes preserve line breaks.

## Layout

The desktop shell has a sticky full-height sidebar (208px), toolbar (80px) and centered content capped at (1120px). Content padding is (36px 32px 48px); sections generally separate by (32px). Rows use bottom dividers and a minimum height of (68px).

At widths up to (760px), the sidebar narrows to (164px), content padding becomes (28px 22px), editors/providers/sessions can wrap, and the next-step container stacks. At widths up to (560px), the shell stacks: the sidebar becomes a top strip with horizontally scrollable navigation, Settings becomes an icon control, and sidebar status/shortcut hints hide. The toolbar becomes (70px), paired fields stack, action rows wrap and content padding becomes (24px 18px 32px). Document minimum width is (360px). These adapt compact desktop windows; they do not establish mobile product scope.

Native window chrome remains outside the web content. Browser preview shows a storage/integration limitation banner. Do not infer native behavior from browser screenshots.

## Elevation & Depth

There are no shadows. Tonal surface steps, borders and spacing convey grouping. Hover uses raised tones; primary-button hover applies brightness (.94). There are no authored transitions or keyframe animations. Reduced-motion CSS disables animation, transitions and smooth scrolling.

## Shapes

Controls and navigation share gently rounded corners; capture is slightly rounder, grouped forms and empty states use panel corners, and status tags use small corners. Dividers and outlines are (1px). Empty states and file-reference drop areas use dashed borders. Inline outline SVG icons use currentColor rather than an icon font.

## Components

### Buttons

Primary actions use accent fill, accent text, weight (500), label size and minimum height (38px). Secondary buttons use surface fill and a border; hover raises their surface. Text buttons underline on hover. Icon buttons have a minimum footprint of (32px), raised hover fill and descriptive title attributes. Disabled controls show opacity (.5) and a not-allowed cursor.

### Inputs / Fields

Fields use canvas fill, border outlines and visible labels. Textareas resize vertically. Focus-visible uses an accent outline (2px) with offset (3px); quick capture instead marks its enclosing border on focus-within. The capture form spans pages; Ctrl/Cmd+K focuses it. Errors use role=alert; saved notices use role=status.

### Navigation

Sidebar buttons use muted text, raised hover fill and accent text when selected. Current pages expose aria-current. Task-type tabs use muted text and an accent underline when active. They expose tablist/tab/aria-selected; a full arrow-key tab pattern is not claimed here.

### Chips / Containers

Status tags are compact bordered text, including explicit disconnected or unverified connection states. Forms use surface-filled panels; lists remain divided rows. Empty states pair an outline icon with a heading and explanation. Loading uses static surface blocks. Error messages add an error border; notices use accent text.

### Verification limits

The review handoff reports the original four browser/UI findings resolved, with desktop, compact and light browser captures under artifacts/. This documentation pass extracted source values; it did not rerun visual or assistive-technology QA. Browser/UI readiness does not establish native release readiness. Native titlebar/resize, global shortcut, tray lifecycle, notifications, sleep/restart recovery, native integrations, Windows scaling and total app/WebView2/worker resource measurements still require native verification. No benchmark or comprehensive accessibility certification is claimed.

## Do's and Don'ts

### Do:

- Do preserve the warm neutral hierarchy and restrained accent.
- Do use visible focus, readable metadata and wrapping for long local paths.
- Do preserve compact window reflow and explicit connection/preview states.

### Don't:

- Don't add custom pet artwork, an animation loop, gradients or remote fonts in this stage. The user-approved native placeholder is functional UI.
- Don't present stored history as verified live agent activity.
- Don't treat browser screenshots as evidence of native release performance or behavior.

## Native primary surfaces (0.4)

The native pet is a static mint placeholder with a pouch/count and remaining file-hold time. It has no idle render loop. A loaded drag copies references outward; Alt-drag moves it. Expiry is two minutes and never deletes originals.

Click opens a native EDIT/results bar near the pet. It uses the existing charcoal/mint palette and Segoe UI through standard Windows controls, with explicit focus, Enter/arrows/Tab/Escape and an IME guard. Prefer below the pet, clamped above when screen edges require it. No WebView is involved in primary commands. Detailed management is an explicit secondary window. Native interaction, high-contrast and mixed-DPI acceptance remain device checks; do not claim visual/keyboard certification from parser tests.


## Task hierarchy, November 2026

Overview contains actionable attention, the next checklist items and short project links. Tasks owns todos, timers and reminders. Projects owns workspace selection and saved next steps. Codex owns explicit requests, decisions and receipts. Browser helper retains capture, review and fill as an explicit sequence. Shelf, Music and Settings keep literal labels.

Project creation, diagnostics and stored history use native disclosure controls. Important pending decisions and uncertain outcomes remain visible. Drafts survive navigation within the current window and stay isolated by form and project. Compact layouts expose every destination through a Tools menu rather than a clipped horizontal strip. Keyboard focus moves to the selected destination; native requests acknowledge only after their target is ready.

See docs/ui-ux-review-2026-11.md for research, the council decision and verification limits.
