# Phase 5 camera backend decision

Date: 2026-11-09

## Decision

Use Nokhwa 0.10.11 for Linux camera discovery and capture, compiled behind the
default-enabled Cargo feature `camera`. Enable only Nokhwa's `input-v4l`
feature. Do not enable its default MJPEG decoder, threaded callback, OpenCV,
GStreamer, or WGPU features.

Nokhwa is preferred over `eye` and framework bindings because it provides the
small cross-device capture API this application needs while retaining a native
V4L2 backend. The application uses Nokhwa's existing `v4l` dependency directly
only during discovery, to reject metadata-only companion nodes by their V4L2
capabilities. Owning format negotiation and streaming directly through `v4l`
would add backend detail, while higher-level multimedia frameworks would add
native dependencies and runtime scope that are unnecessary for a profile-photo
capture window.

The GTK-independent core owns discovery and capture. Device discovery retains
only nodes advertising both video-capture and streaming capabilities, sorts
them by their real V4L2 index rather than assuming enumeration order, and never
starts a stream. Capture runs on a worker thread; GTK consumes only plain frame
data selected for display.

## Capture pipeline

The capture service opens the selected V4L2 device entirely on its worker
thread. It chooses a safe format close to 640 by 480 at 30 frames per second,
preferring MJPEG when otherwise equal. MJPEG frames use the project's existing
bounded JPEG decoder; YUYV, NV12, raw RGB/BGR, and greyscale frames use
Nokhwa's native conversions.

The worker replaces one shared latest-frame slot instead of queueing frames.
The GTK window polls that slot, constructs a texture only for the frame it will
display, and freezes a plain RGB frame when Capture is selected. Cancel, window
close, errors, and application teardown all signal the worker to stop; a
separate shutdown task joins it so GTK never waits for a blocking camera read.
Accepted frames enter the same centre-crop, 512-pixel PNG, atomic `~/.face`,
and AccountsService path used by file selection.

## Licence review

The workspace is `GPL-2.0-or-later`. On 2026-11-09, `cargo metadata
--all-features` was used to inspect the SPDX licence expression for every
resolved package. No package had missing licence metadata, and every expression
offered a GPL-2.0-or-later-compatible choice.

The camera-specific packages are permissively licensed: Nokhwa,
`nokhwa-core`, and `nokhwa-bindings-linux` use Apache-2.0; `v4l` and
`v4l2-sys-mit` use MIT. Their remaining transitive packages use compatible
MIT, Apache-2.0, BSD, ISC, Zlib, or equivalent permissive choices. Apache-2.0
is not compatible with GPLv2-only, but it is compatible here because the
application's `GPL-2.0-or-later` terms permit distribution of the combined work
under GPLv3. See the [GNU licence compatibility
FAQ](https://www.gnu.org/licenses/gpl-faq.html.en) and the [Apache Software
Foundation compatibility note](https://www.apache.org/licenses/GPL-compatibility).

Repeat the resolved-graph review whenever a dependency or enabled feature is
added or updated.
