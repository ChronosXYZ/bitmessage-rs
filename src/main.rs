use async_std::task;
use relm4::{
    gtk::{
        self,
        traits::{BoxExt, GtkWindowExt, OrientableExt},
    },
    view, ComponentParts, RelmApp, RelmWidgetExt, SimpleComponent,
};

mod network;
mod pow;
mod repositories;

struct App {}

#[derive(Debug)]
enum AppInput {}

#[relm4::component]
impl SimpleComponent for App {
    type Input = AppInput;
    type Output = ();
    type Init = ();

    view! {
        gtk::ApplicationWindow {
            set_default_size: (300, 100),

            set_title = Some("Bitmessage-rs"),

            #[wrap(Some)]
            set_titlebar = &gtk::HeaderBar {},

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,

                gtk::Label {
                    set_label: "Hello, World!",
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = App {};

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

fn main() {
    pretty_env_logger::init();

    let (mut client, worker) = network::new(None);

    task::spawn(worker.run());

    task::block_on(client.start_listening("/ip4/0.0.0.0/tcp/0".parse().unwrap()))
        .expect("listening not to fail");

    let app = RelmApp::new("io.github.chronosx88.BitmessageRs");
    app.run::<App>(());
}
