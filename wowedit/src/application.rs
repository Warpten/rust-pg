use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use renderer::gui::context::InterfaceRenderer;
use renderer::orchestration::render::{Renderer, RendererAPI, RendererUpdater};
use wowfs::casc::errors::Error;
use wowfs::fs::FileSystem;
use crate::interface::InterfaceState;
use crate::rendering::geometry::GeometryRenderer;

pub(in crate) struct ApplicationData {
    pub renderer : Renderer,
    pub geometry : GeometryRenderer<SharedState>,
    pub interface : InterfaceRenderer<InterfaceState>,
    pub shared_state : Rc<RefCell<SharedState>>
}

impl ApplicationData {
    fn load_game_install(&mut self, cdn : &str, build: &str, path : &PathBuf) -> Result<(), Error> {
        let fs = match FileSystem::open(path, cdn, build) {
            Ok(fs) => Some(fs),
            Err(err) => return Err(err),
        };

        self.shared_state.borrow_mut().fs = fs;

        Ok(())
    }
}


pub struct SharedState {
    fs : Option<FileSystem>,
}
impl SharedState {
    pub fn load_game_install(&mut self, cdn : &str, build : &str, path : PathBuf) -> bool {
        self.fs = match FileSystem::open(path.parent().unwrap(), build, cdn) {
            Ok(fs) => Some(fs),
            Err(_) => None
        };

        self.fs.is_some()
    }
}
impl Default for SharedState {
    fn default() -> Self {
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
