use async_std::task;
use relm4::gtk::prelude::*;
use relm4::{
    adw,
    gtk::{
        self,
        traits::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    view, ComponentParts, RelmApp, RelmWidgetExt, SimpleComponent,
};
use relm4_icons::icon_name;

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
        adw::ApplicationWindow {
            set_default_size: (400, 300),

            set_title = Some("Bitmessage-rs"),


            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    set_centering_policy: adw::CenteringPolicy::Strict,

                    #[wrap(Some)]
                    #[name="view_title"]
                    set_title_widget = &adw::ViewSwitcherTitle {
                        set_stack: Some(&stack),
                        set_title: "Bitmessage-rs"
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_vexpand: true,

                    #[name="stack"]
                    adw::ViewStack {
                        set_vexpand: true,
                        set_margin_start: 12,
                        set_margin_end: 12,

                        add_titled[Some("identities"), "Identities"] = &gtk::ScrolledWindow {
                            gtk::CenterBox {
                                #[wrap(Some)]
                                set_center_widget = &gtk::Label {
                                    set_label: "No identities yet :(",
                                    add_css_class: "large-title"
                                }
                            }
                        } -> {
                            set_icon_name: Some(icon_name::PERSON),
                        },

                        add_titled[Some("messages"), "Messages"] = &gtk::ScrolledWindow {
                            gtk::CenterBox {
                                #[wrap(Some)]
                                set_center_widget = &gtk::Label {
                                    set_label: "No messages yet :(",
                                    add_css_class: "large-title"
                                }
                            }
                        } -> {
                            set_icon_name: Some(icon_name::MAIL_INBOX_FILLED),
                        },

                        add_titled[Some("status"), "Network Status"] = &gtk::ScrolledWindow {
                            gtk::CenterBox {
                                #[wrap(Some)]
                                set_center_widget = &gtk::Label {
                                    set_label: "No network status yet :(",
                                    add_css_class: "large-title"
                                }
                            }
                        } -> {
                            set_icon_name: Some(icon_name::DESKTOP_PULSE_FILLED),
                        },
                    },

                    #[name = "view_bar"]
                    adw::ViewSwitcherBar {
                        set_stack: Some(&stack),
                    }
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

        widgets
            .view_title
            .bind_property("title-visible", &widgets.view_bar, "reveal")
            .build();

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
    relm4_icons::initialize_icons();
    app.run::<App>(());
}
