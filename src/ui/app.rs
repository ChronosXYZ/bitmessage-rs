use relm4::component::{AsyncComponent, AsyncComponentController, AsyncController};
use relm4::gtk::prelude::*;
use relm4::{adw, gtk, ComponentParts, SimpleComponent};
use relm4_icons::icon_name;

use super::components::identities_list::IdentitiesListModel;
use super::components::messages::MessagesModel;
use super::components::network_status::NetworkStatusModel;

pub(crate) struct AppModel {
    identities_list: AsyncController<IdentitiesListModel>,
    messages: AsyncController<MessagesModel>,
    network_status: AsyncController<NetworkStatusModel>,
}

#[derive(Debug)]
pub(crate) enum AppInput {}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
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

                        add_titled[Some("identities"), "Identities"] = model.identities_list.widget() -> &gtk::ScrolledWindow{} -> {
                            set_icon_name: Some(icon_name::PERSON),
                        },

                        add_titled[Some("messages"), "Messages"] = model.messages.widget() -> &gtk::ScrolledWindow {} -> {
                            set_icon_name: Some(icon_name::MAIL_INBOX_FILLED),
                        },

                        add_titled[Some("status"), "Network Status"] = model.network_status.widget() -> &gtk::ScrolledWindow {} -> {
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
        let identities_list_component = IdentitiesListModel::builder().launch(()).detach();
        let messages_component = MessagesModel::builder().launch(()).detach();
        let network_status_component = NetworkStatusModel::builder().launch(()).detach();

        let model = AppModel {
            identities_list: identities_list_component,
            messages: messages_component,
            network_status: network_status_component,
        };

        let widgets = view_output!();

        widgets
            .view_title
            .bind_property("title-visible", &widgets.view_bar, "reveal")
            .build();

        ComponentParts { model, widgets }
    }
}
