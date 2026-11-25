# Viewing images

Select a PNG or JPEG (`.png`, `.jpg`, or `.jpeg`) and press **F3** to view it
without leaving FFM. The
viewer shows the image's format, dimensions, size and zoom. Use `+` and `-`
to change the zoom, `0` to fit the image again, the arrow keys to move around
a zoomed image, and `Esc` or `F10` to close it.

**Ctrl+Q** turns the other panel into a quick view. It follows the selected
file and shows an image preview and its details. Image loading and preparation
run in the background; an old result is discarded if the selection changes.
Press Ctrl+Q again to restore the file panel.

By default images are drawn with coloured terminal cells, so a preview is
available without terminal graphics support. The result is lower-resolution
than a graphical image. Files that cannot be decoded show an explanation;
to open a file with its associated application, close the viewer and press
Enter on the file.

## High-quality images in VS Code Terminal

VS Code's integrated terminal supports images when its image setting and GPU
acceleration are enabled. For a supported version, set these in VS Code's
`settings.json`:

```json
{
  "terminal.integrated.enableImages": true,
  "terminal.integrated.gpuAcceleration": "auto"
}
```

On Windows, VS Code may also need `terminal.integrated.windowsUseConptyDll`.
Then select the iTerm2 inline image protocol when starting FFM in the terminal:

```sh
FFM_IMAGE_PROTOCOL=iterm2 ffm
```

In PowerShell, set `$env:FFM_IMAGE_PROTOCOL = 'iterm2'` before running `ffm`.
This setting affects FFM's process; it does not change VS Code's settings. If
the terminal does not support this protocol, omit the variable to use the
coloured-cell preview instead. Terminal configuration and support can differ
over SSH, tmux, and Windows.
