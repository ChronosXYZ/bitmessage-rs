use relm4::gtk::prelude::*;
use relm4::RelmWidgetExt;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk,
    loading_widgets::LoadingWidgets,
    view,
};

pub(crate) struct IdentitiesListModel {}

#[derive(Debug)]
pub(crate) enum IdentitiesListInput {}

#[relm4::component(pub async)]
impl AsyncComponent for IdentitiesListModel {
    type CommandOutput = ();
    type Input = IdentitiesListInput;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            gtk::CenterBox {
                #[wrap(Some)]
                set_center_widget = &gtk::Label {
                    set_label: "No identities yet :(",
                    add_css_class: "large-title"
                }
            }
        }
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
        let model = Self {};
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }
}
