use crate::app::AppModel;
use async_std::task;
use directories::ProjectDirs;
use nantoka_core::network;
use relm4::RelmApp;

pub mod app;
mod components;
pub mod state;

fn main() {
    pretty_env_logger::init();

    let dirs = ProjectDirs::from("", "", "bitmessage-rs").unwrap();
    let data_dir = dirs.data_dir();

    let (mut client, worker) = network::new(None, data_dir.to_path_buf());

    task::spawn(worker.run());

    task::block_on(client.start_listening("/ip4/0.0.0.0/tcp/34064".parse().unwrap()))
        .expect("listening not to fail");

    state::STATE.write_inner().client = Some(client);
    relm4::RELM_THREADS.set(4).unwrap();

    let app = RelmApp::new("io.github.chronosx88.BitmessageRs");
    relm4_icons::initialize_icons();
    app.run::<AppModel>(());
}
