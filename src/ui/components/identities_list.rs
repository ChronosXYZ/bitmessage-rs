use relm4::gtk::prelude::*;
use relm4::RelmWidgetExt;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts},
    gtk,
    loading_widgets::LoadingWidgets,
    view,
};

use crate::ui::components::utils::typed_list_view::TypedListView;
use crate::ui::state;

use super::utils::typed_list_view::RelmListItem;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IdentityItem {
    label: String,
    address: String,
}

impl IdentityItem {
    fn new(label: String, address: String) -> Self {
        Self { label, address }
    }
}

struct Widgets {
    label: gtk::Label,
}

impl Drop for Widgets {
    fn drop(&mut self) {
        dbg!(self.label.label());
    }
}

impl RelmListItem for IdentityItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem, column_index: usize) -> (gtk::Box, Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = Widgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root, column_index: usize) {
        let Widgets { label } = widgets;

        if column_index == 0 {
            label.set_label(&self.label);
        } else if column_index == 1 {
            label.set_label(&self.address);
        }
    }
}

pub(crate) struct IdentitiesListModel {
    is_list_empty: bool,
    list_view_wrapper: TypedListView<IdentityItem, gtk::SingleSelection, gtk::ColumnView>,
}

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
                set_center_widget = &gtk::Box{
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_visible: model.is_list_empty,
                        set_spacing: 3,
                        set_valign: gtk::Align::Center,

                        gtk::Label {
                            #[watch]
                            set_label: "No identities yet :(",
                            add_css_class: "large-title"
                        },
                        gtk::Button {
                            set_label: "Create new one",
                            set_hexpand: false,
                        }
                    },

                    #[local_ref]
                    col_view -> gtk::ColumnView {
                        #[watch]
                        set_visible: !model.is_list_empty,
                    }
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
        let list_view_wrapper: TypedListView<IdentityItem, gtk::SingleSelection, gtk::ColumnView> =
            TypedListView::with_sorting_col(vec!["Label".to_string(), "Address".to_string()]);

        let identities = state::STATE
            .write_inner()
            .client
            .as_mut()
            .unwrap()
            .get_own_identities()
            .await;
        let mut is_list_empty = true;
        if !identities.is_empty() {
            is_list_empty = false;
        }

        let model = Self {
            is_list_empty,
            list_view_wrapper,
        };

        let col_view = &model.list_view_wrapper.view;

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }
}
