//! The library Android's NativeActivity loads (`libheron_brush_android.so`): its entry
//! point hands the activity to the shared application in `heron_brush`.

/// Called by the NativeActivity glue (android-activity) on a thread of its own once the
/// activity is created.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: heron_brush::android::AndroidApp) {
    heron_brush::android::run(app);
}
