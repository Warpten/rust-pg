use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use renderer::gui::context::InterfaceRenderer;
use renderer::orchestration::render::{Renderer, RendererAPI, RendererUpdater};
use wowfs::fs::FileSystem;
use crate::async_task_manager::{AsyncTaskManager, AsyncValue};
use crate::interface::InterfaceState;
use crate::rendering::geometry::GeometryRenderer;

pub(in crate) struct ApplicationData {
    pub renderer : Renderer,
    pub geometry : GeometryRenderer<SharedState>,
    pub interface : InterfaceRenderer<InterfaceState>,

    pub shared_state : Arc<RwLock<SharedState>>,
}

pub struct SharedState {
    pub task_mgr : AsyncTaskManager,
    pub fs : AsyncValue<FileSystem>,
}
impl SharedState {
    /// Asynchronously loads a game installation. Returns a receiver that needs to be awaited to retrieve the file system.
    /// 
    /// # Arguments
    ///
    /// * `cdn` - The CDN key of the configuration to load.
    /// * `build` - The build key of the configuration to load.
    /// * `path` - Path on disk of the game installation.
    pub fn async_load_game_install(self : &mut SharedState, cdn : String, build : String, path : PathBuf) {
        let task_handle = self.task_mgr.oneshot_maybe(move || {
            let now = std::time::Instant::now();

            match FileSystem::open(path.as_path(), build, cdn) {
                Ok(value) => {
                    println!("FS loaded in {:.3?}", now.elapsed());
                    Some(value)
                },
                Err(_) => None
            }
        });

        match &self.fs {
            AsyncValue::None => {
                self.fs = AsyncValue::Pending(task_handle)
            },
            AsyncValue::Pending(_) => {
                // Currently loading, abort and replace?
            },
            AsyncValue::Value(_) => {
                // Already loaded, unload?
            },
        };
    }
    
    pub fn default() -> Self {
        Self {
            fs: AsyncValue::None,
            task_mgr : AsyncTaskManager::new()
        }
    }
}

// Rendering specifics below this line

impl ApplicationData {
    pub fn updater(&mut self) -> RendererUpdater {
        self.renderer.updater(vec![
            &mut self.geometry,
            &mut self.interface
        ])
    }
}

impl RendererAPI for ApplicationData {
    fn is_minimized(&self) -> bool { self.renderer.context.window.is_minimized() }
    fn recreate_swapchain(&mut self) {  self.updater().recreate_swapchain()  }
    fn wait_idle(&self) { self.renderer.context.device.wait_idle() }
}
