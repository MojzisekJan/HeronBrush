# Viewport inspection

MatCap and section views help inspect brushwork and imported models while keeping the map and its engine exports unchanged.

## MatCap

Choose **View → MatCap → Clay, Chrome or Normals**, or use **File → Preferences → Renderer → MatCap**. **Off** restores the map materials. The preset is saved in the editor preferences. It uses camera-relative surface normals, so turning or moving the camera changes the captured response. Scene lights, exposure and material textures do not alter the geometry's MatCap appearance.

Clay gives a diffuse surface, Chrome emphasizes changes in curvature, and Normals shows camera-space normal direction as colour. **Renderer → Cavity** remains available. Selection stays visible. Wireframe keeps its usual appearance. MatCap does not replace materials in saved maps, engine exports or saved render-camera output.

## Section view

Toggle **View → Section view**. Open **Map → Section view** in the inspector to change the axis, position and direction. The first enable places the plane through the selection or the map centre. Position uses map units. **Flip section** keeps the other half. **Center section on selection** recentres the plane, using the map centre when nothing is selected.

The section affects the working 3D view, its geometry, overlays and object picking. Flat views keep the complete map. An intersected brush remains an ordinary intact brush; the plane does not create a new brush face. Use the Clip tool when you want to change geometry permanently.

Section settings belong to the working editor session. They do not enter the saved map, collaboration edits, engine exports or Undo history. Turning the section off restores the full view. MatCap can be combined with a section to inspect dark interiors.
