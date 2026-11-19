//! Immutable shader modules shared by live renderers on the same device and thread.
//! Weak entries do not keep a closed renderer or its GPU device alive.

use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};

pub(super) struct ShaderModules {
    device: wgpu::Device,
    pub(super) scene: wgpu::ShaderModule,
    pub(super) shadow: wgpu::ShaderModule,
    pub(super) post: wgpu::ShaderModule,
    pub(super) overlay: wgpu::ShaderModule,
    pub(super) sprite: wgpu::ShaderModule,
    pub(super) ao: wgpu::ShaderModule,
    pub(super) bloom: wgpu::ShaderModule,
    pub(super) ibl: wgpu::ShaderModule,
    pub(super) ssr: wgpu::ShaderModule,
    pub(super) taa: wgpu::ShaderModule,
}

thread_local! {
    static CACHE: RefCell<Vec<Weak<ShaderModules>>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn get(device: &wgpu::Device) -> Arc<ShaderModules> {
    let found = CACHE.with(|cache| {
        let mut entries = cache.borrow_mut();
        entries.retain(|entry| entry.strong_count() > 0);
        entries
            .iter()
            .find_map(|entry| entry.upgrade().filter(|modules| modules.device == *device))
    });
    if let Some(modules) = found {
        return modules;
    }
    // Release the RefCell borrow before calling into the backend.
    let modules = Arc::new(ShaderModules::new(device));
    CACHE.with(|cache| cache.borrow_mut().push(Arc::downgrade(&modules)));
    modules
}

impl ShaderModules {
    fn new(device: &wgpu::Device) -> Self {
        let scene_module = module(
            device,
            "render_wgpu scene",
            &[
                include_str!("shaders/environment.wgsl"),
                include_str!("shaders/lighting.wgsl"),
                include_str!("shaders/material_sample.wgsl"),
                include_str!("shaders/surface_grid.wgsl"),
                include_str!("shaders/scene.wgsl"),
            ],
        );
        let shadow_module = module(
            device,
            "render_wgpu shadow",
            &[
                include_str!("shaders/material_sample.wgsl"),
                include_str!("shaders/shadow.wgsl"),
            ],
        );
        let post_module = module(
            device,
            "render_wgpu post",
            &[
                include_str!("shaders/surface_grid.wgsl"),
                include_str!("shaders/post.wgsl"),
            ],
        );
        let overlay_module = module(
            device,
            "render_wgpu overlay",
            &[include_str!("shaders/overlay.wgsl")],
        );
        let sprite_module = module(
            device,
            "render_wgpu sprite",
            &[include_str!("shaders/sprite.wgsl")],
        );
        let ao_module = module(device, "render_wgpu ao", &[include_str!("shaders/ao.wgsl")]);
        let bloom_module = module(
            device,
            "render_wgpu bloom",
            &[include_str!("shaders/bloom.wgsl")],
        );
        let ibl_module = module(
            device,
            "render_wgpu ibl",
            &[
                include_str!("shaders/environment.wgsl"),
                include_str!("shaders/ibl.wgsl"),
            ],
        );
        let ssr_module = module(
            device,
            "render_wgpu ssr",
            &[
                include_str!("shaders/lighting.wgsl"),
                include_str!("shaders/ssr.wgsl"),
            ],
        );
        let taa_module = module(
            device,
            "render_wgpu taa",
            &[include_str!("shaders/taa.wgsl")],
        );

        Self {
            device: device.clone(),
            scene: scene_module,
            shadow: shadow_module,
            post: post_module,
            overlay: overlay_module,
            sprite: sprite_module,
            ao: ao_module,
            bloom: bloom_module,
            ibl: ibl_module,
            ssr: ssr_module,
            taa: taa_module,
        }
    }
}

const COMMON: &str = include_str!("shaders/common.wgsl");

fn module(device: &wgpu::Device, label: &str, sources: &[&str]) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(crate::cube_array::shader_source(format!("{COMMON}\n{}", sources.join("\n"))).into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_are_device_scoped_and_released_with_the_last_owner() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .expect("shader cache test requires a GPU adapter");
        let (device, _) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let (other_device, _) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let first = get(&device);
        let second = get(&device);
        assert!(Arc::ptr_eq(&first, &second));
        let other = get(&other_device);
        assert!(!Arc::ptr_eq(&first, &other), "modules cannot cross devices");
        let weak = Arc::downgrade(&first);
        drop(first);
        assert!(
            weak.upgrade().is_some(),
            "the second renderer still needs its modules"
        );
        drop(second);
        assert!(
            weak.upgrade().is_none(),
            "the cache must not retain GPU resources"
        );
        let reopened = get(&device);
        assert!(Arc::ptr_eq(&reopened, &get(&device)));
        assert!(Arc::ptr_eq(&other, &get(&other_device)));
    }
}
