use crate::ui::app::AppModel;
use async_std::task;
use relm4::RelmApp;
use ui::state;

mod network;
mod pow;
mod repositories;
mod ui;

fn main() {
    pretty_env_logger::init();

    let (mut client, worker) = network::new(None);

    task::spawn(worker.run());

    task::block_on(client.start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap()))
        .expect("listening not to fail");

    state::STATE.write_inner().client = Some(client);

    let app = RelmApp::new("io.github.chronosx88.BitmessageRs");
    relm4_icons::initialize_icons();
    app.run::<AppModel>(());
}
