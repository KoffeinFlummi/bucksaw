use std::{future::Future, sync::mpsc::Receiver};

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_in_background<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
pub fn execute_in_background<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub struct BackgroundCompStore<C> {
    data: Option<C>,
    receiver: Receiver<C>,
}

impl<C> BackgroundCompStore<C> {
    // None -> nothing has arrived to the receiver side. C instantation is still in progress
    pub fn get(&mut self) -> &Option<C> {
        if let Ok(data) = self.receiver.try_recv() {
            self.data = Some(data);
        }
        &self.data
    }

    pub fn new(receiver: Receiver<C>) -> Self {
        Self {
            data: None,
            receiver,
        }
    }
}
