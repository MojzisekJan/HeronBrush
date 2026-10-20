# Render cameras and PNG output

Open **Render image…** in the main menu. Position the normal 3D view and click
**Place camera from 3D view**. This copies its position, orientation and vertical
field of view. The preview uses the chosen output aspect ratio.

The camera is a selectable point object in the **Render cameras** layer, with a
gold frustum in the 3D editor. Move it with the regular tools; the render window automatically updates its
controls and preview. Alternatively,
edit X/Y/Z, yaw and pitch directly in that window and click **Save camera and
settings**. Positions use map units. **Move 3D view to camera** moves the editor's
viewpoint. Enable **Camera frame in 3D view** to see the output boundary with
darkened surroundings and matching field of view, regardless of viewport shape.

Enable **Control camera from 3D view** to compose the shot using ordinary viewport
navigation. It creates a camera if needed and updates that camera as you move.
Changes enter Undo after a 350 ms pause, with the last movement also saved when
you disable the link or close the window. Selecting another object does not
redirect the active navigation link. Deleting its camera stops the link.

Save the map to persist the camera and its render settings. If several cameras
exist, select the desired one before opening or reloading the render window;
otherwise the window keeps its current camera, or initially uses the first one. A camera can be duplicated with the regular
duplicate command. Camera edits use the ordinary undo history. The generated
camera layer is marked omitted from game exports; leave that flag enabled.

Output controls include width, height, samples, vertical field of view, far clip,
exposure, sun direction, shadows, ambient occlusion and bloom. Expand **Lighting**
to change sky, horizon, ground and sun colours, direct and ambient light intensity,
shadow softness and AO strength. These controls update the preview immediately;
**Save camera and settings** stores them with the camera. Older cameras initially
inherit these values from editor preferences. Material defaults and the remaining
renderer options still come from those preferences.

The **Daylight**, **Sunset** and **Overcast** buttons set environment colours,
light intensity, shadow softness, AO strength and sun direction. They leave camera
position, exposure and effect toggles as chosen. **Full HD**, **Square** and
**Portrait** set output dimensions; you can still enter custom width and height.
Camera position, output/lens and lighting controls are collapsible sections.
Choose **Render PNG…** and a destination file. PNG dimensions are independent of
the preview/window size and desktop scaling. Supported dimensions are 16–4096
pixels per axis, subject to the GPU's framebuffer limits; samples range from 1–64.

Choose **Render GIF…** under the preview to render the map's animations
as a looping GIF. The render starts at time zero and plays everything that starts on
its own: keyframe animations with **start_on** set, `func_rotate` and
`func_shuttle`. Brushes waiting for a trigger stay still, and entity models do not
move. **Length (seconds)** and **Frames per second** set how many frames are rendered;
Both are in the **Animation** section. **Length from animation** sets the length to one cycle of the longest animation.
Each frame is a full render with the chosen number of samples, so a 4 second clip at
25 frames per second takes 100 times as long as one PNG. GIF frame delays are whole
hundredths of a second, so the rate is capped at 50 frames per second, and each frame
is reduced to 256 colours. A Full HD clip of a few seconds can take tens of megabytes;
use a smaller output size for sharing.

Expand **Depth of field** and enable it to blur objects outside the focus region.
**Focus distance** is measured along the camera's forward axis in map units.
**Sharp range** is the full interval around that distance kept sharp; set it to
zero for a single focus plane. **Blur radius** controls the maximum blur in pixels
at 1080 pixels output height, scaling with resolution so the composition stays
consistent between preview and PNG. Select an object in the map and use
**Focus on selection** to focus on its bounding-box centre. Objects behind the
camera cannot be used. Save camera settings and the map to preserve the focus.

This is a depth-buffer effect, not a physical lens simulation. It cannot recover
hidden surfaces, and overlapping silhouettes or transparent materials can show
artifacts. It is disabled by default, including in older saved cameras.

This uses the existing OpenGL raster renderer, including its materials and face
colour gradients. It is not a path tracer. The image contains visible brushes,
patches and loaded entity models, without selection tint, locked-layer tint,
axes, gizmos or placeholder entity boxes. Editor billboard sprites are omitted.
The PNG has an opaque background. Rendering refines the preview one sample at a
time and displays sample progress. **Cancel render** stops between samples without
writing an image; starting again discards the cancelled accumulation. Geometry
and settings are captured at render start.

Rendering runs on the editor's UI thread and graphics context: an individual GPU
sample/readback and final PNG encoding can briefly pause interaction, especially at
large resolutions.
There is currently no animation rendering.

The render window is exercised by the UI tests
in `crates/heron-brush` (`cargo test -p heron-brush`), which open every dialog; the
rendered output and camera navigation are checked by hand.
