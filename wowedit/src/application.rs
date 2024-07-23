use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use futures::TryFutureExt;
use renderer::gui::context::InterfaceRenderer;
use renderer::orchestration::render::{Renderer, RendererAPI, RendererUpdater};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use wowfs::fs::FileSystem;
use crate::interface::InterfaceState;
use crate::rendering::geometry::GeometryRenderer;

pub(in crate) struct ApplicationData {
    pub renderer : Renderer,
    pub geometry : GeometryRenderer<SharedState>,
    pub interface : InterfaceRenderer<InterfaceState>,
    pub shared_state : Arc<Mutex<SharedState>>,
}

pub struct SharedState {
    fs : Option<FileSystem>,
}
impl SharedState {
    /// Asynchronously loads a game installation. Returns a receiver that needs to be awaited to retrieve the file system.
    /// 
    /// # Arguments
    /// 
    /// * `runtime` - The Tokio runtime in charge of running the CASC filesystem loading task.
    /// * `cdn` - The CDN key of the configuration to load.
    /// * `build` - The build key of the configuration to load.
    /// * `path` - Path on disk of the game installation.
    pub fn async_load_game_install(self : &mut SharedState, runtime: &Runtime, cdn : String, build : String, path : PathBuf) {
        let (tx, mut rx) = oneshot::channel();
        runtime.spawn(async move {
            match FileSystem::open(path, build, cdn) {
                Ok(fs) => { _ = tx.send(fs); }, // We don't care if the listener gave up
                Err(_) => { },
            };
        });

        let fs = rx.try_recv();
    }
    
    pub fn default() -> Self {
        Self { fs: None }
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
