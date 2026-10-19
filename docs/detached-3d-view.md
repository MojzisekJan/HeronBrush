# Separate 3D window

Enable **3D view in separate window** in the main menu. Drag the new window to
another monitor and resize or maximize it normally. The main window uses the
available space for the remaining 2D views. Its pane-layout preference still
controls whether these are shown together or cycled with Space.

This moves the existing perspective widget: camera position, selection, tools,
render-camera framing and the document are shared. Editing, grid, save and open
shortcuts are installed in both windows. Preferences can also be opened from the
secondary window with the usual shortcut.

Use **Return to main window**, disable the menu toggle, or close the secondary
window to dock the view again. Closing the main map window also closes the
secondary window; cancelling the main window's save confirmation leaves both open.
Window placement is managed by the operating system, not saved across app launches.

The window is a second egui viewport (`crates/heron-brush/src/detached.rs`) drawing
the same 3D view and renderer as the docked pane. Detaching, docking and pane-layout
changes are covered by the manual checks.

Multi-monitor and mixed-DPI behaviour is checked by hand.

The detached window uses the operating system's standard window controls, with no
manually duplicated minimise/maximise buttons. Its header has touch-sized **Materials**, **Solid view**
and **Wireframe** buttons, plus **Render image…** for camera/render settings and PNG
output. The render dialog opens relative to the window containing the 3D view.

Wireframe is also available in the main view-filter panel. It shows triangle edges
(including triangulation diagonals and edges behind surfaces) without filled faces,
textures, sprites or post-processing. Tool handles remain visible. Its line colour
contrasts with the configured background. The mode is a temporary viewport setting;
it does not change map data, exports or final image rendering.

Use **Ctrl+Alt+Num0** (or **Ctrl+Alt+0** without a numeric keypad) to align the
selected render camera to the current perspective view. With no selected camera,
the first saved camera is updated; if none exists, one is created. The same command
is available in the main menu as **Place camera from 3D view**. Both map windows
support the shortcut, in either keyboard preset. The change can be undone, and
existing output resolution, lighting and depth-of-field settings are retained.
