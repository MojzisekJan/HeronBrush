# iOS drawable geometry

Based on winit 0.30.13. `inner_size` and `inner_position` in
`src/platform_impl/ios/window.rs` use the full screen frame instead of subtracting
safe-area insets. UIKit renders and delivers touch coordinates over this full view;
`layoutSubviews` already emits resize events with these dimensions. egui-winit
provides safe-area insets separately. Using the smaller size makes eframe configure
a smaller Metal surface and stretches the interface relative to touch targets.

The unused safe-area helper and its OS-capability entry were removed.
Only the iOS backend is changed. Remove this patch when upstream reports consistent
drawable geometry. Validate menu hit targets and saving in both iPad orientations.
