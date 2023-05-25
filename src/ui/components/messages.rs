use adw;
use gtk::{self, prelude::*};
use relm4::component::{AsyncComponentController, AsyncController};
use relm4::RelmWidgetExt;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    loading_widgets::LoadingWidgets,
    view,
};

use super::messages_sidebar::MessagesSidebar;

pub(crate) struct MessagesModel {
    sidebar: AsyncController<MessagesSidebar>,
}

#[derive(Debug)]
pub(crate) enum MessagesInput {}

#[relm4::component(pub async)]
impl AsyncComponent for MessagesModel {
    type CommandOutput = ();
    type Input = MessagesInput;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            adw::Leaflet {
                model.sidebar.widget() -> &gtk::ScrolledWindow,
                gtk::Separator {},
                gtk::Box {
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    gtk::Label {
                        set_halign: gtk::Align::Center,
                        set_label: "No messages yet :(",
                        add_css_class: "large-title"
                    }
                }
            }
        }


        //gtk::ScrolledWindow {
        //    gtk::CenterBox {
        //        #[wrap(Some)]
        //        set_center_widget = &gtk::Label {
        //            set_label: "No messages yet :(",
        //            add_css_class: "large-title"
        //        }
        //    }
        //}
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
                #[local_ref]
                root {
                    #[name(loading)]
                    gtk::CenterBox {
                        set_margin_all: 100,
                        set_orientation: gtk::Orientation::Vertical,
                        #[wrap(Some)]
                        set_center_widget = &gtk::Spinner {
                            start: (),
                            set_size_request: (40, 40),
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                        },
                    }
                }
        }
        Some(LoadingWidgets::new(root, loading))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let sidebar = MessagesSidebar::builder().launch(()).detach();
        let model = Self { sidebar };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }
}
