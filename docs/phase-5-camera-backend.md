# Phase 5 camera backend decision

Date: 2026-11-09

## Decision

Use Nokhwa 0.10.11 for Linux camera discovery and capture, compiled behind the
default-enabled Cargo feature `camera`. Enable only Nokhwa's `input-v4l`
feature. Do not enable its default MJPEG decoder, threaded callback, OpenCV,
GStreamer, or WGPU features.

Nokhwa is preferred over `eye`, direct `v4l`, and framework bindings because it
provides the small cross-device API this application needs while retaining a
native V4L2 backend. Direct `v4l` would make the application own more format
negotiation and backend detail, while the higher-level multimedia frameworks
would add native dependencies and runtime scope that are unnecessary for a
profile-photo capture window.

The GTK-independent core owns discovery and capture. Device discovery returns
the real indices reported by V4L2 rather than assuming `/dev/video0`. Capture
will run on a worker thread in the next checkpoint; GTK will only consume plain
frame data selected for display.

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
