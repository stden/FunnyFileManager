# Image viewing and panel usability

FFM should make image files visible from the existing F3 viewer and Ctrl+Q
quick-view panel while keeping the Far-style keyboard workflow. PNG and JPEG
are the first supported formats. The fullscreen viewer fits the image and
shows its format, dimensions, and file size, with controls for zoom and
navigation; quick view follows the selected file without blocking the UI.

Image decoding is bounded and happens away from the UI event loop. A result
for an old selection must never replace the current preview. On a terminal
with supported graphics, allow explicitly selecting a graphics protocol;
otherwise show a safe fallback. Handle inaccessible,
unsupported, malformed, and excessively large files without leaving stale
graphics or blocking navigation. Resize, overlays, and closing the viewer
must restore the underlying screen correctly.

VS Code Terminal is the primary target. Its image support depends on the user's
terminal settings; FFM must remain usable when it is disabled. Existing text
viewing, keyboard bindings, mouse hit testing, and other terminal types must
continue to work.

Improve discovery of the existing quick view in F1, adapt function-key labels
to the available width without changing hit regions, display active filters
clearly, and distinguish an empty folder from an empty filtered result. Ctrl+I
must allow editing and applying the include and exclude masks and hidden-file
option; cancelling or failing to reload must preserve the previous panel state.

Verify file-type and bounds handling, decoding and errors, rendering and
resizing, cancellation and stale results, viewer controls, quick-view state,
small terminals, filter and empty states, and existing key and mouse behavior.
Run the repository lint and full test suite on the integrated tree, and check
the supported terminal behavior with a PTY or equivalent when possible.
