use std::{
    collections::HashMap,
    sync::mpsc::{self},
};

use rand::prelude::*;
use winit::event_loop::EventLoopProxy;

use crate::{util::UserEvent, vec2::Vec2};

trait UiClosure: Fn(&mut egui::Ui) -> bool + Send {}
impl<T: Fn(&mut egui::Ui) -> bool + Send> UiClosure for T {}

struct Popup {
    name: &'static str,
    closure: Box<dyn UiClosure>,
}

pub struct PopupManager {
    popups: HashMap<usize, Popup>,
    receiver: mpsc::Receiver<Popup>,
    proxy: PopupProxy,
}

impl PopupManager {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let popups = HashMap::new();
        let (sender, receiver) = mpsc::channel();

        Self {
            popups,
            receiver,
            proxy: PopupProxy { sender, proxy },
        }
    }

    pub fn get_proxy(&self) -> PopupProxy {
        self.proxy.clone()
    }

    pub fn update(&mut self, ctx: &egui::Context, size: Vec2<f32>) {
        let mut rng = thread_rng();
        while let Ok(popup) = self.receiver.try_recv() {
            self.popups.insert(rng.gen(), popup);
        }

        for key in self.popups.keys().copied().collect::<Vec<_>>() {
            let popup = &self.popups[&key];
            let mut open = true;
            let mut done = false;
            egui::Window::new(popup.name)
                .id(egui::Id::new(key))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(size / 2.0)
                .open(&mut open)
                .show(ctx, |ui| done = (popup.closure)(ui));
            if !open || done {
                self.popups.remove(&key);
            }
        }
    }
}

#[derive(Clone)]
pub struct PopupProxy {
    sender: mpsc::Sender<Popup>,
    proxy: EventLoopProxy<UserEvent>,
}

impl PopupProxy {
    pub fn spawn_popup<T, F>(&self, name: &'static str, closure: F) -> PopupHandle<T>
    where
        F: Fn(&mut egui::Ui) -> Option<T> + Send + 'static,
        T: 'static + Send,
    {
        let (tx, rx) = mpsc::channel();
        let outer = move |ui: &mut egui::Ui| {
            let value = (closure)(ui);
            match value {
                Some(value) => {
                    let _ = tx.send(value);
                    true
                }
                None => false,
            }
        };

        let popup = Popup {
            name,
            closure: Box::new(outer),
        };

        let _ = self.sender.send(popup);
        let _ = self.proxy.send_event(UserEvent::Wake);
        PopupHandle(rx)
    }
}

pub struct PopupHandle<T>(mpsc::Receiver<T>);

impl<T> PopupHandle<T> {
    pub fn wait(self) -> Option<T> {
        self.0.recv().ok()
    }
}
