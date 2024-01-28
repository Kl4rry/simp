use std::{
    collections::HashMap,
    sync::mpsc::{self},
};

use cgmath::{EuclideanSpace, Point2, Vector2};
use rand::prelude::*;
use winit::event_loop::EventLoopProxy;

use crate::util::{p2, UserEvent};

trait UiClosure: Fn(&mut egui::Ui) -> bool + Send {}
impl<T: Fn(&mut egui::Ui) -> bool + Send> UiClosure for T {}

struct Dialog {
    name: &'static str,
    closure: Box<dyn UiClosure>,
}

pub struct DialogManager {
    dialogs: HashMap<usize, Dialog>,
    receiver: mpsc::Receiver<Dialog>,
    proxy: DialogProxy,
}

impl DialogManager {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let dialogs = HashMap::new();
        let (sender, receiver) = mpsc::channel();

        Self {
            dialogs,
            receiver,
            proxy: DialogProxy { sender, proxy },
        }
    }

    pub fn get_proxy(&self) -> DialogProxy {
        self.proxy.clone()
    }

    pub fn update(&mut self, ctx: &egui::Context, size: Vector2<f32>) {
        let mut rng = thread_rng();
        while let Ok(dialog) = self.receiver.try_recv() {
            self.dialogs.insert(rng.gen(), dialog);
        }

        for key in self.dialogs.keys().copied().collect::<Vec<_>>() {
            let dialog = &self.dialogs[&key];
            let mut open = true;
            let mut done = false;
            egui::Window::new(dialog.name)
                .id(egui::Id::new(key))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(p2(Point2::from_vec(size / 2.0)))
                .open(&mut open)
                .show(ctx, |ui| done = (dialog.closure)(ui));
            if !open || done {
                self.dialogs.remove(&key);
            }
        }
    }
}

#[derive(Clone)]
pub struct DialogProxy {
    sender: mpsc::Sender<Dialog>,
    proxy: EventLoopProxy<UserEvent>,
}

impl DialogProxy {
    pub fn spawn_dialog<T, F>(&self, name: &'static str, closure: F) -> DialogHandle<T>
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

        let dialog = Dialog {
            name,
            closure: Box::new(outer),
        };

        let _ = self.sender.send(dialog);
        let _ = self.proxy.send_event(UserEvent::Wake);
        DialogHandle(rx)
    }
}

pub struct DialogHandle<T>(mpsc::Receiver<T>);

impl<T> DialogHandle<T> {
    pub fn wait(self) -> Option<T> {
        self.0.recv().ok()
    }
}
